use crate::numeric::{Ratio, UsdBtc, CKBTC, TAL};
use crate::vault::Vault;
use crate::{
    compute_collateral_ratio, InitArg, ProtocolError, UpgradeArg, MINIMUM_COLLATERAL_RATIO,
    RECOVERY_COLLATERAL_RATIO,
};
use candid::Principal;
use ic_canister_log::log;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Serialize;
use std::cell::RefCell;
use std::cmp::max;
use std::collections::btree_map::Entry::{Occupied, Vacant};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

// Like assert_eq, but returns an error instead of panicking.
macro_rules! ensure_eq {
    ($lhs:expr, $rhs:expr, $msg:expr $(, $args:expr)* $(,)*) => {
        if $lhs != $rhs {
            return Err(format!("{} ({:?}) != {} ({:?}): {}",
                               std::stringify!($lhs), $lhs,
                               std::stringify!($rhs), $rhs,
                               format!($msg $(,$args)*)));
        }
    }
}

macro_rules! ensure {
    ($cond:expr, $msg:expr $(, $args:expr)* $(,)*) => {
        if !$cond {
            return Err(format!("Condition {} is false: {}",
                               std::stringify!($cond),
                               format!($msg $(,$args)*)));
        }
    }
}

/// Controls which operations the protocol can perform.
#[derive(candid::CandidType, Clone, Debug, PartialEq, Eq, serde::Deserialize, Serialize, Copy)]
pub enum Mode {
    /// Protocol's state is read-only.
    ReadOnly,
    /// No restrictions on the protocol interactions.
    GeneralAvailability,
    /// The protocols tries to get back to a total
    /// collateral ratio above 150%
    Recovery,
}

pub const CKBTC_TRANSFER_FEE: CKBTC = CKBTC::new(10);

impl Mode {
    pub fn is_available(&self) -> bool {
        match self {
            Mode::ReadOnly => false,
            Mode::GeneralAvailability => true,
            Mode::Recovery => true,
        }
    }

    pub fn get_minimum_liquidation_collateral_ratio(&self) -> Ratio {
        match self {
            Mode::ReadOnly => MINIMUM_COLLATERAL_RATIO,
            Mode::GeneralAvailability => MINIMUM_COLLATERAL_RATIO,
            Mode::Recovery => RECOVERY_COLLATERAL_RATIO,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mode::ReadOnly => write!(f, "Read-only"),
            Mode::GeneralAvailability => write!(f, "General availability"),
            Mode::Recovery => write!(f, "Recovery"),
        }
    }
}

impl Default for Mode {
    fn default() -> Self {
        Self::GeneralAvailability
    }
}

pub type VaultId = u64;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, Serialize, Copy)]
pub struct PendingMarginTransfer {
    pub owner: Principal,
    pub margin: CKBTC,
}

thread_local! {
    static __STATE: RefCell<Option<State>> = RefCell::default();
}

pub const DEFAULT_BORROW_FEE: Ratio = Ratio::new(dec!(0.005));

pub struct State {
    /// Maps vault id to vault.
    pub vault_id_to_vaults: BTreeMap<u64, Vault>,
    /// Maps vault owner to vault ids.
    pub principal_to_vault_ids: BTreeMap<Principal, BTreeSet<u64>>,
    /// Maps liquidity provider to provided TAL liquidity amount.
    pub liquidity_pool: BTreeMap<Principal, TAL>,
    /// Liquidity Pool retruns (in ckBTC)
    pub liquidity_returns: BTreeMap<Principal, CKBTC>,

    pub pending_margin_transfers: BTreeMap<VaultId, PendingMarginTransfer>,
    pub pending_redemption_transfer: BTreeMap<u64, PendingMarginTransfer>,
    pub last_redemption_time: u64,
    pub current_base_rate: Ratio,
    /// The mode in which the protocol runs.
    pub mode: Mode,

    /// The fee charged when borrowing: e8s.
    pub fee: Ratio,

    pub developer_principal: Principal,

    pub next_available_vault_id: u64,

    pub total_collateral_ratio: Ratio,

    /// Principal of the exchange rate canister.
    /// https://wiki.internetcomputer.org/wiki/Exchange_rate_canister.
    pub xrc_principal: Principal,
    /// Pincipal of the TAL ledger canister.
    pub taler_ledger_principal: Principal,
    /// Principal of the ckBTC ledger canister.
    pub ckbtc_ledger_principal: Principal,
    pub ckbtc_ledger_fee: CKBTC,
    /// Last Bitcoin rate fetched from XRC.
    pub last_btc_rate: Option<UsdBtc>,
    /// Last timestamp of fetch Bitcoin rate.
    pub last_btc_timestamp: Option<u64>,

    /// Guards
    pub principal_guards: BTreeSet<Principal>,
    pub is_timer_running: bool,
    pub is_fetching_rate: bool,
}

impl From<InitArg> for State {
    fn from(args: InitArg) -> Self {
        let fee = Decimal::from_u64(args.fee_e8s).unwrap() / dec!(100_000_000);
        Self {
            last_redemption_time: 0,
            current_base_rate: Ratio::from(Decimal::ZERO),
            fee: Ratio::from(fee),
            developer_principal: args.developer_principal,
            principal_to_vault_ids: BTreeMap::new(),
            pending_redemption_transfer: BTreeMap::new(),
            vault_id_to_vaults: BTreeMap::new(),
            xrc_principal: args.xrc_principal,
            taler_ledger_principal: args.taler_ledger_principal,
            ckbtc_ledger_principal: args.ckbtc_ledger_principal,
            ckbtc_ledger_fee: CKBTC_TRANSFER_FEE,
            mode: Mode::GeneralAvailability,
            total_collateral_ratio: Ratio::from(Decimal::MAX),
            last_btc_timestamp: None,
            last_btc_rate: None,
            next_available_vault_id: 0,
            liquidity_pool: BTreeMap::new(),
            liquidity_returns: BTreeMap::new(),
            principal_guards: BTreeSet::new(),
            pending_margin_transfers: BTreeMap::new(),
            is_timer_running: false,
            is_fetching_rate: false,
        }
    }
}

impl State {
    pub fn check_price_not_too_old(&self) -> Result<(), ProtocolError> {
        let current_time = ic_cdk::api::time();
        const TEN_MINS_NANOS: u64 = 10 * 60 * 1_000_000_000;
        let last_btc_timestamp = match self.last_btc_timestamp {
            Some(last_btc_timestamp) => last_btc_timestamp,
            None => {
                return Err(ProtocolError::TemporarilyUnavailable(
                    "No BTC price fetched".to_string(),
                ))
            }
        };
        if current_time.saturating_sub(last_btc_timestamp) > TEN_MINS_NANOS {
            log!(
                crate::INFO,
                "No recent price entry switching protocol to readonly mode, lastest: {}, current time: {current_time}", last_btc_timestamp
            );
            return Err(ProtocolError::TemporarilyUnavailable(
                "Last known BTC price too old".to_string(),
            ));
        }
        Ok(())
    }

    pub fn increment_vault_id(&mut self) -> u64 {
        let vault_id = self.next_available_vault_id;
        self.next_available_vault_id += 1;
        vault_id
    }

    pub fn upgrade(&mut self, args: UpgradeArg) {
        if let Some(mode) = args.mode {
            self.mode = mode;
        }
    }

    pub fn total_borrowed_tal_amount(&self) -> TAL {
        self.vault_id_to_vaults
            .values()
            .map(|vault| vault.borrowed_tal_amount)
            .sum()
    }

    pub fn total_ckbtc_margin_amount(&self) -> CKBTC {
        self.vault_id_to_vaults
            .values()
            .map(|vault| vault.ckbtc_margin_amount)
            .sum()
    }

    pub fn compute_total_collateral_ratio(&self, btc_rate: UsdBtc) -> Ratio {
        if self.total_borrowed_tal_amount() == 0 {
            return Ratio::from(Decimal::MAX);
        }
        (self.total_ckbtc_margin_amount() * btc_rate) / self.total_borrowed_tal_amount()
    }

    pub fn get_redemption_fee(&self, redeemed_amount: TAL) -> Ratio {
        let current_time = ic_cdk::api::time();
        let last_redemption_time = self.last_redemption_time;
        let elapsed_hours = (current_time - last_redemption_time) / 1_000_000_000 / 3600;
        compute_redemption_fee(
            elapsed_hours,
            redeemed_amount,
            self.total_borrowed_tal_amount(),
            self.current_base_rate,
        )
    }

    pub fn get_borrowing_fee(&self) -> Ratio {
        match self.mode {
            Mode::Recovery => Ratio::from(Decimal::ZERO),
            Mode::GeneralAvailability => self.fee,
            Mode::ReadOnly => self.fee,
        }
    }

    pub fn update_total_collateral_ratio_and_mode(&mut self, btc_rate: UsdBtc) {
        let previous_mode = self.mode;
        let new_total_collateral_ratio = self.compute_total_collateral_ratio(btc_rate);
        self.total_collateral_ratio = new_total_collateral_ratio;
        if new_total_collateral_ratio < crate::RECOVERY_COLLATERAL_RATIO {
            self.mode = Mode::Recovery;
        } else {
            self.mode = Mode::GeneralAvailability
        }
        if new_total_collateral_ratio < Ratio::from(dec!(1.0)) {
            self.mode = Mode::ReadOnly
        }
        if previous_mode != self.mode {
            log!(
                crate::DEBUG,
                "[update_total_collateral_ratio_and_mode] switched mode to {}, current total collateral ratio: {}, minimum collateral ratio {:?}",
                self.mode,
                new_total_collateral_ratio.to_f64(),
                self.mode.get_minimum_liquidation_collateral_ratio().to_f64()
            );
        }
    }

    pub fn open_vault(&mut self, vault: Vault) {
        let vault_id = vault.vault_id;
        self.vault_id_to_vaults.insert(vault_id, vault.clone());
        match self.principal_to_vault_ids.get_mut(&vault.owner) {
            Some(vault_ids) => {
                vault_ids.insert(vault_id);
            }
            None => {
                let mut vault_ids: BTreeSet<u64> = BTreeSet::new();
                vault_ids.insert(vault_id);
                self.principal_to_vault_ids.insert(vault.owner, vault_ids);
            }
        }
    }

    pub fn close_vault(&mut self, vault_id: u64) {
        if let Some(vault) = self.vault_id_to_vaults.remove(&vault_id) {
            let owner = vault.owner;
            self.pending_margin_transfers.insert(
                vault_id,
                PendingMarginTransfer {
                    owner,
                    margin: vault.ckbtc_margin_amount,
                },
            );
            if let Some(vault_ids) = self.principal_to_vault_ids.get_mut(&owner) {
                vault_ids.remove(&vault_id);
            } else {
                ic_cdk::trap("BUG: tried to close vault with no owner");
            }
        } else {
            ic_cdk::trap("BUG: tried to close unknown vault");
        }
    }

    pub fn borrow_from_vault(&mut self, vault_id: u64, borrowed_amount: TAL) {
        match self.vault_id_to_vaults.get_mut(&vault_id) {
            Some(vault) => {
                vault.borrowed_tal_amount += borrowed_amount;
            }
            None => ic_cdk::trap("borrowing from unkown vault"),
        }
    }

    pub fn add_margin_to_vault(&mut self, vault_id: u64, add_margin: CKBTC) {
        match self.vault_id_to_vaults.get_mut(&vault_id) {
            Some(vault) => {
                vault.ckbtc_margin_amount += add_margin;
            }
            None => ic_cdk::trap("adding margin to unkown vault"),
        }
    }

    pub fn repay_to_vault(&mut self, vault_id: u64, repayed_amount: TAL) {
        match self.vault_id_to_vaults.get_mut(&vault_id) {
            Some(vault) => {
                assert!(repayed_amount <= vault.borrowed_tal_amount);
                vault.borrowed_tal_amount -= repayed_amount;
            }
            None => ic_cdk::trap("repaying to unkown vault"),
        }
    }

    pub fn provide_liquidity(&mut self, amount: TAL, caller: Principal) {
        if amount == 0 {
            return;
        }
        self.liquidity_pool
            .entry(caller)
            .and_modify(|curr| *curr += amount)
            .or_insert(amount);
    }

    pub fn withdraw_liquidity(&mut self, amount: TAL, caller: Principal) {
        match self.liquidity_pool.entry(caller) {
            Occupied(mut entry) => {
                assert!(*entry.get() >= amount);
                *entry.get_mut() -= amount;
                if *entry.get() == 0 {
                    entry.remove_entry();
                }
            }
            Vacant(_) => ic_cdk::trap("cannot remove liquidity from unknow principal"),
        }
    }

    pub fn claim_liquidity_returns(&mut self, amount: CKBTC, caller: Principal) {
        match self.liquidity_returns.entry(caller) {
            Occupied(mut entry) => {
                assert!(*entry.get() >= amount);
                *entry.get_mut() -= amount;
                if *entry.get() == 0 {
                    entry.remove_entry();
                }
            }
            Vacant(_) => ic_cdk::trap("cannot claim returns from unknow principal"),
        }
    }

    pub fn get_liquidity_returns_of(&self, principal: Principal) -> CKBTC {
        *self.liquidity_returns.get(&principal).unwrap_or(&0.into())
    }

    pub fn total_provided_liquidity_amount(&self) -> TAL {
        self.liquidity_pool.values().cloned().sum()
    }

    pub fn total_available_returns(&self) -> CKBTC {
        self.liquidity_returns.values().cloned().sum()
    }

    pub fn get_provided_liquidity(&self, principal: Principal) -> TAL {
        *self.liquidity_pool.get(&principal).unwrap_or(&TAL::from(0))
    }

    pub fn liquidate_vault(&mut self, vault_id: u64, mode: Mode, btc_rate: UsdBtc) {
        let vault = self
            .vault_id_to_vaults
            .get(&vault_id)
            .cloned()
            .expect("bug: vault not found");
        assert!(self.total_provided_liquidity_amount() >= vault.borrowed_tal_amount);
        let vault_collateral_ratio = compute_collateral_ratio(&vault, btc_rate);
        let entries = if mode == Mode::Recovery && vault_collateral_ratio > MINIMUM_COLLATERAL_RATIO
        {
            let partial_margin = (vault.borrowed_tal_amount * MINIMUM_COLLATERAL_RATIO) / btc_rate;
            assert!(
                partial_margin <= vault.ckbtc_margin_amount,
                "partial margin: {partial_margin}, vault margin: {}",
                vault.ckbtc_margin_amount
            );
            match self.vault_id_to_vaults.get_mut(&vault_id) {
                Some(vault) => {
                    vault.borrowed_tal_amount = 0.into();
                    assert!(vault.ckbtc_margin_amount >= partial_margin);
                    vault.ckbtc_margin_amount -= partial_margin;
                }
                None => ic_cdk::trap("liquidating unkown vault"),
            }
            log!(
                crate::DEBUG,
                "[liquidate_vault] Do not liquidate totally as CR still above 110%",
            );
            distribute_across_lps(
                &self.liquidity_pool,
                vault.borrowed_tal_amount,
                partial_margin,
            )
        } else {
            if let Some(vault) = self.vault_id_to_vaults.remove(&vault_id) {
                if let Some(vault_ids) = self.principal_to_vault_ids.get_mut(&vault.owner) {
                    vault_ids.remove(&vault_id);
                }
            }
            distribute_across_lps(
                &self.liquidity_pool,
                vault.borrowed_tal_amount,
                vault.ckbtc_margin_amount,
            )
        };
        assert!(!entries.is_empty());
        for entry in entries {
            match self.liquidity_pool.entry(entry.owner) {
                Occupied(mut lp_entry) => {
                    log!(
                        crate::DEBUG,
                        "[liquidate_vault] tal to debit: {}",
                        entry.tal_to_debit,
                    );
                    assert!(
                        *lp_entry.get() >= entry.tal_to_debit,
                        "entry contains {} cannot substract {}",
                        *lp_entry.get(),
                        entry.tal_to_debit
                    );
                    *lp_entry.get_mut() -= entry.tal_to_debit;
                    if *lp_entry.get() == 0 {
                        lp_entry.remove();
                    }
                }
                Vacant(_) => {
                    ic_cdk::trap("bug: principal not found in liquidity_pool");
                }
            }
            self.liquidity_returns
                .entry(entry.owner)
                .and_modify(|v| *v += entry.ckbtc_reward)
                .or_insert(entry.ckbtc_reward);
        }
    }

    pub fn redistribute_vault(&mut self, vault_id: u64) {
        let vault = self
            .vault_id_to_vaults
            .get(&vault_id)
            .expect("bug: vault not found");
        let entries = distribute_accross_vaults(&self.vault_id_to_vaults, vault.clone());
        for entry in entries {
            match self.vault_id_to_vaults.entry(entry.vault_id) {
                Occupied(mut vault_entry) => {
                    vault_entry.get_mut().ckbtc_margin_amount += entry.ckbtc_share_amount;
                    vault_entry.get_mut().borrowed_tal_amount += entry.tal_share_amount;
                }
                Vacant(_) => panic!("bug: vault not found"),
            }
        }
        if let Some(vault) = self.vault_id_to_vaults.remove(&vault_id) {
            let owner = vault.owner;
            if let Some(vault_ids) = self.principal_to_vault_ids.get_mut(&owner) {
                vault_ids.remove(&vault_id);
            }
        }
    }

    pub fn redeem_on_vaults(&mut self, tal_amount: TAL, current_btc_rate: UsdBtc) {
        let mut tal_amount_to_convert = tal_amount;
        let mut vaults: BTreeSet<(Ratio, VaultId)> = BTreeSet::new();

        for vault in self.vault_id_to_vaults.values() {
            vaults.insert((
                crate::compute_collateral_ratio(vault, current_btc_rate),
                vault.vault_id,
            ));
        }

        let vault_ids: Vec<VaultId> = vaults.iter().map(|(_cr, vault_id)| *vault_id).collect();
        let mut index: usize = 0;

        while tal_amount_to_convert > 0 && index < vault_ids.len() {
            let vault = self.vault_id_to_vaults.get(&vault_ids[index]).unwrap();

            if vault.borrowed_tal_amount >= tal_amount_to_convert {
                // We can convert everything on this vault
                let redeemable_ckbtc_amount: CKBTC = tal_amount_to_convert / current_btc_rate;
                self.deduct_amount_from_vault(
                    redeemable_ckbtc_amount,
                    tal_amount_to_convert,
                    vault_ids[index],
                );
                break;
            } else {
                // Convert what we can on this vault
                let redeemable_tal_amount = vault.borrowed_tal_amount;
                let redeemable_ckbtc_amount: CKBTC = redeemable_tal_amount / current_btc_rate;
                self.deduct_amount_from_vault(
                    redeemable_ckbtc_amount,
                    redeemable_tal_amount,
                    vault_ids[index],
                );
                tal_amount_to_convert -= redeemable_tal_amount;
                index += 1;
            }
        }
        debug_assert!(tal_amount_to_convert == 0);
    }

    fn deduct_amount_from_vault(
        &mut self,
        ckbtc_amount_to_deduct: CKBTC,
        tal_amount_to_deduct: TAL,
        vault_id: VaultId,
    ) {
        match self.vault_id_to_vaults.get_mut(&vault_id) {
            Some(vault) => {
                assert!(vault.borrowed_tal_amount >= tal_amount_to_deduct);
                vault.borrowed_tal_amount -= tal_amount_to_deduct;
                assert!(vault.ckbtc_margin_amount >= ckbtc_amount_to_deduct);
                vault.ckbtc_margin_amount -= ckbtc_amount_to_deduct;
            }
            None => ic_cdk::trap("cannot deduct from unknown vault"),
        }
    }

    /// Checks whether the internal state of the core canister matches the other state
    /// semantically (the state holds the same data, but maybe in a slightly
    /// different form).
    pub fn check_semantically_eq(&self, other: &Self) -> Result<(), String> {
        let mut vault_id_to_vaults = self.vault_id_to_vaults.clone();
        let mut other_vault_id_to_vaults = other.vault_id_to_vaults.clone();
        vault_id_to_vaults.retain(|k, vault| {
            if let Some(other_vault) = other.vault_id_to_vaults.get(k) {
                return vault == other_vault && !other.vault_id_to_vaults.contains_key(k);
            }
            false
        });
        other_vault_id_to_vaults.retain(|k, _| !self.vault_id_to_vaults.contains_key(k));
        ensure_eq!(
            vault_id_to_vaults.len(),
            0,
            "vault_id_to_vaults does not match"
        );
        ensure_eq!(
            other_vault_id_to_vaults.len(),
            0,
            "vault_id_to_vaults does not match"
        );

        ensure_eq!(
            self.vault_id_to_vaults,
            other.vault_id_to_vaults,
            "vault_id_to_vaults does not match"
        );
        ensure_eq!(
            self.pending_margin_transfers,
            other.pending_margin_transfers,
            "pending_margin_transfers does not match"
        );
        ensure_eq!(
            self.principal_to_vault_ids,
            other.principal_to_vault_ids,
            "principal_to_vault_ids does not match"
        );
        ensure_eq!(
            self.liquidity_pool,
            other.liquidity_pool,
            "liquidity_pool does not match"
        );
        ensure_eq!(
            self.liquidity_returns,
            other.liquidity_returns,
            "liquidity_returns does not match"
        );
        ensure_eq!(
            self.xrc_principal,
            other.xrc_principal,
            "xrc_principal does not match"
        );
        ensure_eq!(
            self.taler_ledger_principal,
            other.taler_ledger_principal,
            "taler_ledger_principal does not match"
        );
        ensure_eq!(
            self.ckbtc_ledger_principal,
            other.ckbtc_ledger_principal,
            "ckbtc_ledger_principal does not match"
        );
        ensure_eq!(
            self.pending_redemption_transfer,
            other.pending_redemption_transfer,
            "pending_redemption_transfer does not match"
        );

        Ok(())
    }

    pub fn check_invariants(&self) -> Result<(), String> {
        ensure!(
            self.vault_id_to_vaults.len()
                <= self
                    .principal_to_vault_ids
                    .values()
                    .map(|set| set.len())
                    .sum::<usize>(),
            "Inconsistent TAL: burned {}, minted: {}",
            self.vault_id_to_vaults.len(),
            self.principal_to_vault_ids
                .values()
                .map(|set| set.len())
                .sum::<usize>(),
        );

        for vault_ids in self.principal_to_vault_ids.values() {
            for vault_id in vault_ids {
                if self.vault_id_to_vaults.get(vault_id).is_none() {
                    panic!("Not all vault ids are in the id -> Vault map.")
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct DistributeEntry {
    pub owner: Principal,
    pub ckbtc_reward: CKBTC,
    pub tal_to_debit: TAL,
}

/// Liquidate a vault by liquidity providers.
/// Hypothesis: sum(provided_liquidity) >= vault.tal
pub(crate) fn distribute_across_lps(
    provided_liquidity: &BTreeMap<Principal, TAL>,
    borrowed_tal_amount: TAL,
    ckbtc_margin_amount: CKBTC,
) -> Vec<DistributeEntry> {
    let total_provided_amount: TAL = provided_liquidity.values().cloned().sum();
    assert!(total_provided_amount >= borrowed_tal_amount);

    let mut result: Vec<DistributeEntry> = vec![];

    for (owner, provided_amount) in provided_liquidity {
        let share: Ratio = *provided_amount / total_provided_amount;
        let ckbtc_reward = ckbtc_margin_amount * share;
        let tal_to_debit = borrowed_tal_amount * share;
        assert!(tal_to_debit <= *provided_amount);
        log!(
            crate::DEBUG,
            "[distribute_across_lps] tal_to_debit: {tal_to_debit}, provided amount: {} for a reward of {ckbtc_reward} and owner: {}",
            *provided_amount,
            *owner
        );

        result.push(DistributeEntry {
            owner: *owner,
            ckbtc_reward,
            tal_to_debit,
        });
    }

    let total_borrowed_tal_amount = result.iter().map(|e| e.tal_to_debit).sum();
    assert!(borrowed_tal_amount >= total_borrowed_tal_amount);
    result[0].tal_to_debit += borrowed_tal_amount - total_borrowed_tal_amount;

    let total_ckbtc_reward = result.iter().map(|e| e.ckbtc_reward).sum();
    assert!(ckbtc_margin_amount >= total_ckbtc_reward);
    result[0].ckbtc_reward += ckbtc_margin_amount - total_ckbtc_reward;

    assert_eq!(
        result.iter().map(|e| e.ckbtc_reward).sum::<CKBTC>(),
        ckbtc_margin_amount
    );

    result
}

pub(crate) struct DistributeToVaultEntry {
    pub vault_id: u64,
    pub ckbtc_share_amount: CKBTC,
    pub tal_share_amount: TAL,
}

/// Liquidate a vault by distributing debt and margin to other vaults
/// Hypothesis: The system should have at least one vault.
pub(crate) fn distribute_accross_vaults(
    vaults: &BTreeMap<u64, Vault>,
    target_vault: Vault,
) -> Vec<DistributeToVaultEntry> {
    assert!(!vaults.is_empty());

    let target_vault_id = target_vault.vault_id;
    let total_ckbtc_margin_amount: CKBTC = vaults
        .iter()
        .filter(|&(&vault_id, _vault)| vault_id != target_vault_id)
        .map(|(_vault_id, vault)| vault.ckbtc_margin_amount)
        .sum();
    assert_ne!(total_ckbtc_margin_amount, 0_u64);

    let mut result: Vec<DistributeToVaultEntry> = vec![];
    let mut distributed_ckbtc: CKBTC = 0_u64.into();
    let mut distributed_tal: TAL = 0_u64.into();

    for (vault_id, vault) in vaults {
        if *vault_id != target_vault_id {
            let share: Ratio = vault.ckbtc_margin_amount / total_ckbtc_margin_amount;
            let ckbtc_share_amount = target_vault.ckbtc_margin_amount * share;
            let tal_share_amount = target_vault.borrowed_tal_amount * share;
            distributed_ckbtc += ckbtc_share_amount;
            distributed_tal += tal_share_amount;
            result.push(DistributeToVaultEntry {
                vault_id: *vault_id,
                ckbtc_share_amount,
                tal_share_amount,
            })
        }
    }
    result[0].tal_share_amount += max(
        target_vault
            .borrowed_tal_amount
            .saturating_sub(distributed_tal),
        distributed_tal.saturating_sub(target_vault.borrowed_tal_amount),
    );
    result[0].ckbtc_share_amount += max(
        target_vault
            .ckbtc_margin_amount
            .saturating_sub(distributed_ckbtc),
        distributed_ckbtc.saturating_sub(target_vault.ckbtc_margin_amount),
    );

    result
}

fn compute_redemption_fee(
    elapsed_hours: u64,
    redeemed_amount: TAL,
    total_borrowed_tal_amount: TAL,
    current_base_rate: Ratio,
) -> Ratio {
    if total_borrowed_tal_amount == 0 {
        return Ratio::from(Decimal::ZERO);
    }
    const REEDEMED_PROPORTION: Ratio = Ratio::new(dec!(0.5)); // 0.5
    const DECAY_FACTOR: Ratio = Ratio::new(dec!(0.94));

    log!(
        crate::INFO,
        "current_base_rate: {current_base_rate}, elapsed_hours: {elapsed_hours}"
    );

    let rate = current_base_rate * DECAY_FACTOR.pow(elapsed_hours);
    let total_rate = rate + redeemed_amount / total_borrowed_tal_amount * REEDEMED_PROPORTION;
    debug_assert!(total_rate < Ratio::from(dec!(1.0)));
    total_rate
        .max(Ratio::from(dec!(0.005)))
        .min(Ratio::from(dec!(0.05)))
}

/// Take the current state.
///
/// After calling this function the state won't be initialized anymore.
/// Panics if there is no state.
pub fn take_state<F, R>(f: F) -> R
where
    F: FnOnce(State) -> R,
{
    __STATE.with(|s| f(s.take().expect("State not initialized!")))
}

/// Mutates (part of) the current state using `f`.
///
/// Panics if there is no state.
pub fn mutate_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut State) -> R,
{
    __STATE.with(|s| f(s.borrow_mut().as_mut().expect("State not initialized!")))
}

/// Read (part of) the current state using `f`.
///
/// Panics if there is no state.
pub fn read_state<F, R>(f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    __STATE.with(|s| f(s.borrow().as_ref().expect("State not initialized!")))
}

/// Replaces the current state.
pub fn replace_state(state: State) {
    __STATE.with(|s| {
        *s.borrow_mut() = Some(state);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::{CKBTC, TAL};
    use std::collections::BTreeMap;

    #[test]
    fn test_distribute_across_on_lp() {
        // Define input data
        let mut provided_liquidity = BTreeMap::new();
        provided_liquidity.insert(Principal::anonymous(), TAL::from(500_000)); // Example liquidity provided
                                                                               // Call the function
        let result = distribute_across_lps(
            &provided_liquidity,
            TAL::from(500_000),
            CKBTC::from(1_000_000),
        );

        // Assertions
        assert_eq!(result.len(), 1); // Assuming there's only one LP in the input

        let distribute_entry = &result[0];
        assert_eq!(distribute_entry.ckbtc_reward, CKBTC::from(1_000_000));
        assert_eq!(distribute_entry.tal_to_debit, TAL::from(500_000));
    }

    #[test]
    fn test_distribute_accross_vaults() {
        // Define input data
        let mut vaults = BTreeMap::new();
        let vault1 = Vault {
            owner: Principal::anonymous(),
            vault_id: 1,
            ckbtc_margin_amount: CKBTC::from(500_000),
            borrowed_tal_amount: TAL::from(300_000),
        };
        let vault2 = Vault {
            owner: Principal::anonymous(),
            vault_id: 2,
            ckbtc_margin_amount: CKBTC::from(300_000),
            borrowed_tal_amount: TAL::from(200_000),
        };
        vaults.insert(1, vault1);
        vaults.insert(2, vault2);

        let target_vault = Vault {
            owner: Principal::anonymous(),
            vault_id: 3,
            ckbtc_margin_amount: CKBTC::from(700_000),
            borrowed_tal_amount: TAL::from(400_000),
        };

        // Call the function
        let result = distribute_accross_vaults(&vaults, target_vault);

        // Assertions
        assert_eq!(result.len(), 2); // Assuming there are two vaults other than the target vault

        let distribute_entry1 = &result[0];
        assert_eq!(distribute_entry1.ckbtc_share_amount, CKBTC::from(437_500)); // Example calculated ckbtc_share_amount
        assert_eq!(distribute_entry1.tal_share_amount, TAL::from(250_000)); // Example calculated tal_share_amount

        let distribute_entry2 = &result[1];
        assert_eq!(distribute_entry2.ckbtc_share_amount, CKBTC::from(262_500)); // Example calculated ckbtc_share_amount
        assert_eq!(distribute_entry2.tal_share_amount, TAL::from(150_000)); // Example calculated tal_share_amount
    }

    #[test]
    fn should_compute_redemption_fee() {
        use crate::E8S;

        let elapsed_hours: u64 = 2;
        let redeemed_amount = TAL::from(150_000 * E8S);
        let total_borrowed_tal_amount = TAL::from(10_000_000 * E8S);
        let current_base_rate = Ratio::from(dec!(0.014)); // 1.4%
        let result = compute_redemption_fee(
            elapsed_hours,
            redeemed_amount,
            total_borrowed_tal_amount,
            current_base_rate,
        );
        assert_eq!(result, Ratio::from(dec!(0.0198704)));
    }

    #[test]
    fn max_redemption_fee() {
        use crate::E8S;

        let elapsed_hours: u64 = 0;
        let redeemed_amount = TAL::from(150_000 * E8S);
        let total_borrowed_tal_amount = TAL::from(10_000_000 * E8S);
        let current_base_rate = Ratio::from(dec!(0.05)); // 5%
        let result = compute_redemption_fee(
            elapsed_hours,
            redeemed_amount,
            total_borrowed_tal_amount,
            current_base_rate,
        );
        assert_eq!(result, Ratio::from(dec!(0.05)));
    }

    #[test]
    fn min_redemption_fee() {
        use crate::E8S;

        let elapsed_hours: u64 = 1_000_000;
        let redeemed_amount = TAL::from(E8S);
        let total_borrowed_tal_amount = TAL::from(10_000_000 * E8S);
        let current_base_rate = Ratio::from(dec!(0.05)); // 5%
        let result = compute_redemption_fee(
            elapsed_hours,
            redeemed_amount,
            total_borrowed_tal_amount,
            current_base_rate,
        );
        assert_eq!(result, Ratio::from(dec!(0.005)));
    }
}
