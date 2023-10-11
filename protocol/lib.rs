use crate::event::{record_liquidate_vault, record_redistribute_vault};
use crate::guard::GuardError;
use crate::logs::{DEBUG, INFO};
use crate::numeric::{Ratio, UsdBtc, CKBTC, TAL};
use crate::state::{mutate_state, read_state, Mode};
use crate::vault::Vault;
use candid::{CandidType, Deserialize, Principal};
use ic_canister_log::log;
use icrc_ledger_types::icrc1::transfer::TransferError;
use icrc_ledger_types::icrc2::transfer_from::TransferFromError;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Serialize;

pub mod dashboard;
pub mod event;
pub mod guard;
pub mod liquidity_pool;
pub mod logs;
pub mod management;
pub mod numeric;
pub mod state;
pub mod storage;
pub mod vault;
pub mod xrc;

#[cfg(test)]
mod tests;

pub const SEC_NANOS: u64 = 1_000_000_000;
pub const E8S: u64 = 100_000_000;

pub const MIN_LIQUIDITY_AMOUNT: TAL = TAL::new(1_000_000_000);
pub const MIN_CKBTC_AMOUNT: CKBTC = CKBTC::new(100_000);
pub const MIN_TAL_AMOUNT: TAL = TAL::new(1_000_000_000);

pub const RECOVERY_COLLATERAL_RATIO: Ratio = Ratio::new(dec!(1.5));
pub const MINIMUM_COLLATERAL_RATIO: Ratio = Ratio::new(dec!(1.1));

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolArg {
    Init(InitArg),
    Upgrade(UpgradeArg),
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitArg {
    pub xrc_principal: Principal,
    pub taler_ledger_principal: Principal,
    pub ckbtc_ledger_principal: Principal,
    pub fee_e8s: u64,
    pub developer_principal: Principal,
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpgradeArg {
    pub mode: Option<Mode>,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct ProtocolStatus {
    pub last_btc_rate: f64,
    pub last_btc_timestamp: u64,
    pub total_ckbtc_margin: u64,
    pub total_tal_borrowed: u64,
    pub total_collateral_ratio: f64,
    pub mode: Mode,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct Fees {
    pub borrowing_fee: f64,
    pub redemption_fee: f64,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct SuccessWithFee {
    pub block_index: u64,
    pub fee_amount_paid: u64,
}

#[derive(candid::CandidType, Deserialize)]
pub struct GetEventsArg {
    pub start: u64,
    pub length: u64,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct LiquidityStatus {
    pub liquidity_provided: u64,
    pub total_liquidity_provided: u64,
    pub liquidity_pool_share: f64,
    pub available_liquidity_reward: u64,
    pub total_available_returns: u64,
}

#[derive(CandidType, Debug, Clone, Deserialize)]
pub enum ProtocolError {
    TransferFromError(TransferFromError, u64),
    TransferError(TransferError),
    TemporarilyUnavailable(String),
    AlreadyProcessing,
    AnonymousCallerNotAllowed,
    CallerNotOwner,
    AmountTooLow { minimum_amount: u64 },
    GenericError(String),
}

impl From<GuardError> for ProtocolError {
    fn from(e: GuardError) -> Self {
        match e {
            GuardError::AlreadyProcessing => Self::AlreadyProcessing,
            GuardError::TooManyConcurrentRequests => {
                Self::TemporarilyUnavailable("too many concurrent requests".to_string())
            }
        }
    }
}

pub fn check_vaults() {
    let last_btc_rate = read_state(|s| s.last_btc_rate.expect("unknown btc rate"));
    let (unhealthy_vaults, healthy_vault) = read_state(|s| {
        let mut unhealthy_vaults: Vec<Vault> = vec![];
        let mut healthy_vault: Vec<Vault> = vec![];
        for vault in s.vault_id_to_vaults.values() {
            if compute_collateral_ratio(vault, last_btc_rate)
                < s.mode.get_minimum_liquidation_collateral_ratio()
            {
                unhealthy_vaults.push(vault.clone());
            } else {
                healthy_vault.push(vault.clone())
            }
        }
        (unhealthy_vaults, healthy_vault)
    });
    for vault in unhealthy_vaults {
        let provided_liquidity = read_state(|s| s.total_provided_liquidity_amount());
        if vault.borrowed_tal_amount <= provided_liquidity {
            log!(
                INFO,
                "[check_vaults] liquidate vault {:?} to liquidity pool with liquidity: {} TAL",
                vault.clone(),
                provided_liquidity
            );
            mutate_state(|s| record_liquidate_vault(s, vault.vault_id, s.mode, last_btc_rate));
        } else if !healthy_vault.is_empty() {
            log!(
                INFO,
                "[check_vaults] redistribute vault {:?} to all the other vaults.",
                vault.clone()
            );
            mutate_state(|s| record_redistribute_vault(s, vault.vault_id));
        } else if read_state(|s| s.total_collateral_ratio) > Ratio::from(dec!(1.0)) {
            log!(
                    INFO,
                    "[check_vaults] cannot liquidate vault {:?} not changing mode as protocol is still solvable, will retry later.",
                    vault.clone(),
                );
        } else {
            log!(
                INFO,
                "[check_vaults] cannot liquidate vault {:?} switching to read-only.",
                vault.clone(),
            );
            mutate_state(|s| s.mode = Mode::ReadOnly);
        }
    }
}

pub fn compute_collateral_ratio(vault: &Vault, btc_rate: UsdBtc) -> Ratio {
    if vault.borrowed_tal_amount == 0 {
        return Ratio::from(Decimal::MAX);
    }
    let margin_value: TAL = vault.ckbtc_margin_amount * btc_rate;
    margin_value / vault.borrowed_tal_amount
}

pub(crate) async fn process_pending_transfer() {
    use crate::state::PendingMarginTransfer;

    let _guard = match crate::guard::TimerLogicGuard::new() {
        Some(guard) => guard,
        None => {
            log!(INFO, "[process_pending_transfer] double entry.",);
            return;
        }
    };

    let pending_transfers = read_state(|s| {
        s.pending_margin_transfers
            .iter()
            .map(|(vault_id, margin_transfer)| (*vault_id, *margin_transfer))
            .collect::<Vec<(u64, PendingMarginTransfer)>>()
    });
    let ckbtc_transfer_fee = read_state(|s| s.ckbtc_ledger_fee);
    for (vault_id, transfer) in pending_transfers {
        match crate::management::transfer_ckbtc(
            transfer.margin - ckbtc_transfer_fee,
            transfer.owner,
        )
        .await
        {
            Ok(block_index) => {
                log!(
                    INFO,
                    "[transfering_margins] successfully transfered: {} to {}",
                    transfer.margin,
                    transfer.owner
                );
                mutate_state(|s| crate::event::record_margin_transfer(s, vault_id, block_index));
            }
            Err(error) => log!(
                DEBUG,
                "[transfering_margins] failed to transfer margin: {}, with error: {}",
                transfer.margin,
                error
            ),
        }
    }

    let pending_transfers = read_state(|s| {
        s.pending_redemption_transfer
            .iter()
            .map(|(block_index, margin_transfer)| (*block_index, *margin_transfer))
            .collect::<Vec<(u64, PendingMarginTransfer)>>()
    });

    for (tal_block_index, pending_transfer) in pending_transfers {
        match crate::management::transfer_ckbtc(
            pending_transfer.margin - ckbtc_transfer_fee,
            pending_transfer.owner,
        )
        .await
        {
            Ok(block_index) => {
                log!(
                    INFO,
                    "[transfering_redemptions] successfully transfered: {} to {}",
                    pending_transfer.margin,
                    pending_transfer.owner
                );
                mutate_state(|s| {
                    crate::event::record_redemption_transfered(s, tal_block_index, block_index)
                });
            }
            Err(error) => log!(
                DEBUG,
                "[transfering_redemptions] failed to transfer margin: {}, with error: {}",
                pending_transfer.margin,
                error
            ),
        }
    }

    if read_state(|s| {
        !s.pending_margin_transfers.is_empty() || !s.pending_redemption_transfer.is_empty()
    }) {
        ic_cdk_timers::set_timer(std::time::Duration::from_secs(1), || {
            ic_cdk::spawn(crate::process_pending_transfer())
        });
    }
}
