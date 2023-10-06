use crate::numeric::{UsdBtc, CKBTC, TAL};
use crate::state::{PendingMarginTransfer, State};
use crate::storage::record_event;
use crate::vault::Vault;
use crate::{InitArg, Mode, UpgradeArg};
use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    #[serde(rename = "open_vault")]
    OpenVault { vault: Vault, block_index: u64 },

    #[serde(rename = "close_vault")]
    CloseVault {
        vault_id: u64,
        block_index: Option<u64>,
    },

    #[serde(rename = "margin_transfer")]
    MarginTransfer { vault_id: u64, block_index: u64 },

    #[serde(rename = "liquidate_vault")]
    LiquidateVault {
        vault_id: u64,
        mode: Mode,
        btc_rate: UsdBtc,
    },

    #[serde(rename = "redemption_on_vaults")]
    RedemptionOnVaults {
        owner: Principal,
        current_btc_rate: UsdBtc,
        tal_amount: TAL,
        fee_amount: TAL,
        tal_block_index: u64,
    },

    #[serde(rename = "redemption_transfered")]
    RedemptionTransfered {
        tal_block_index: u64,
        ckbtc_block_index: u64,
    },

    #[serde(rename = "redistribute_vault")]
    RedistributeVault { vault_id: u64 },

    #[serde(rename = "borrow_from_vault")]
    BorrowFromVault {
        vault_id: u64,
        borrowed_amount: TAL,
        fee_amount: TAL,
        block_index: u64,
    },

    #[serde(rename = "repay_to_vault")]
    RepayToVault {
        vault_id: u64,
        repayed_amount: TAL,
        block_index: u64,
    },

    #[serde(rename = "add_margin_to_vault")]
    AddMarginToVault {
        vault_id: u64,
        margin_added: CKBTC,
        block_index: u64,
    },

    #[serde(rename = "provide_liquidity")]
    ProvideLiquidity {
        amount: TAL,
        block_index: u64,
        caller: Principal,
    },

    #[serde(rename = "withdraw_liquidity")]
    WithdrawLiquidity {
        amount: TAL,
        block_index: u64,
        caller: Principal,
    },

    #[serde(rename = "claim_liquidity_returns")]
    ClaimLiquidityReturns {
        amount: CKBTC,
        block_index: u64,
        caller: Principal,
    },

    #[serde(rename = "init")]
    Init(InitArg),

    #[serde(rename = "upgrade")]
    Upgrade(UpgradeArg),
}

impl Event {
    // Define a method to check if the event contains vault_id
    pub fn is_vault_related(&self, filter_vault_id: &u64) -> bool {
        match self {
            Event::OpenVault { vault, .. } => &vault.vault_id == filter_vault_id,
            Event::CloseVault { vault_id, .. } => vault_id == filter_vault_id,
            Event::MarginTransfer { vault_id, .. } => vault_id == filter_vault_id,
            Event::LiquidateVault { vault_id, .. } => vault_id == filter_vault_id,
            Event::RedemptionOnVaults { .. } => true,
            Event::RedemptionTransfered { .. } => false,
            Event::RedistributeVault { vault_id, .. } => vault_id == filter_vault_id,
            Event::BorrowFromVault { vault_id, .. } => vault_id == filter_vault_id,
            Event::RepayToVault { vault_id, .. } => vault_id == filter_vault_id,
            Event::AddMarginToVault { vault_id, .. } => vault_id == filter_vault_id,
            Event::ProvideLiquidity { .. } => false,
            Event::WithdrawLiquidity { .. } => false,
            Event::ClaimLiquidityReturns { .. } => false,
            Event::Init(_) => false,
            Event::Upgrade(_) => false,
        }
    }
}

#[derive(Debug)]
pub enum ReplayLogError {
    /// There are no events in the event log.
    EmptyLog,
    /// The event log is inconsistent.
    InconsistentLog(String),
}

pub fn replay(mut events: impl Iterator<Item = Event>) -> Result<State, ReplayLogError> {
    let mut state = match events.next() {
        Some(Event::Init(args)) => State::from(args),
        Some(evt) => {
            return Err(ReplayLogError::InconsistentLog(format!(
                "The first event is not Init: {:?}",
                evt
            )))
        }
        None => return Err(ReplayLogError::EmptyLog),
    };
    let mut vault_id = 0;
    for event in events {
        match event {
            Event::OpenVault {
                vault,
                block_index: _,
            } => {
                vault_id += 1;
                state.open_vault(vault);
            }
            Event::CloseVault {
                vault_id,
                block_index: _,
            } => state.close_vault(vault_id),
            Event::LiquidateVault {
                vault_id,
                mode,
                btc_rate,
            } => state.liquidate_vault(vault_id, mode, btc_rate),
            Event::RedistributeVault { vault_id } => state.redistribute_vault(vault_id),
            Event::BorrowFromVault {
                vault_id,
                borrowed_amount,
                fee_amount,
                block_index: _,
            } => {
                state.provide_liquidity(fee_amount, state.developer_principal);
                state.borrow_from_vault(vault_id, borrowed_amount)
            }
            Event::RedemptionOnVaults {
                owner,
                current_btc_rate,
                tal_amount,
                fee_amount,
                tal_block_index,
            } => {
                state.provide_liquidity(fee_amount, state.developer_principal);
                state.redeem_on_vaults(tal_amount, current_btc_rate);
                let margin: CKBTC = tal_amount / current_btc_rate;
                state
                    .pending_redemption_transfer
                    .insert(tal_block_index, PendingMarginTransfer { owner, margin });
            }
            Event::RedemptionTransfered {
                tal_block_index, ..
            } => {
                state.pending_redemption_transfer.remove(&tal_block_index);
            }
            Event::AddMarginToVault {
                vault_id,
                margin_added,
                ..
            } => state.add_margin_to_vault(vault_id, margin_added),
            Event::RepayToVault {
                vault_id,
                repayed_amount,
                ..
            } => {
                state.repay_to_vault(vault_id, repayed_amount);
            }
            Event::ProvideLiquidity { amount, caller, .. } => {
                state.provide_liquidity(amount, caller);
            }
            Event::WithdrawLiquidity { amount, caller, .. } => {
                state.withdraw_liquidity(amount, caller);
            }
            Event::ClaimLiquidityReturns { amount, caller, .. } => {
                state.claim_liquidity_returns(amount, caller);
            }
            Event::Init(_) => panic!("should have only one init event"),
            Event::Upgrade(upgrade_args) => {
                state.upgrade(upgrade_args);
            }
            Event::MarginTransfer { vault_id, .. } => {
                state.pending_margin_transfers.remove(&vault_id);
            }
        }
    }
    state.next_available_vault_id = vault_id;
    Ok(state)
}

pub fn record_liquidate_vault(state: &mut State, vault_id: u64, mode: Mode, btc_rate: UsdBtc) {
    record_event(&Event::LiquidateVault {
        vault_id,
        mode,
        btc_rate,
    });
    state.liquidate_vault(vault_id, mode, btc_rate);
}

pub fn record_redistribute_vault(state: &mut State, vault_id: u64) {
    record_event(&Event::RedistributeVault { vault_id });
    state.redistribute_vault(vault_id);
}

pub fn record_provide_liquidity(
    state: &mut State,
    amount: TAL,
    caller: Principal,
    block_index: u64,
) {
    record_event(&Event::ProvideLiquidity {
        amount,
        block_index,
        caller,
    });
    state.provide_liquidity(amount, caller);
}

pub fn record_withdraw_liquidity(
    state: &mut State,
    amount: TAL,
    caller: Principal,
    block_index: u64,
) {
    record_event(&Event::WithdrawLiquidity {
        amount,
        block_index,
        caller,
    });
    state.withdraw_liquidity(amount, caller);
}

pub fn record_claim_liquidity_returns(
    state: &mut State,
    amount: CKBTC,
    caller: Principal,
    block_index: u64,
) {
    record_event(&Event::ClaimLiquidityReturns {
        amount,
        block_index,
        caller,
    });
    state.claim_liquidity_returns(amount, caller);
}

pub fn record_open_vault(state: &mut State, vault: Vault, block_index: u64) {
    record_event(&Event::OpenVault {
        vault: vault.clone(),
        block_index,
    });
    state.open_vault(vault);
}

pub fn record_close_vault(state: &mut State, vault_id: u64, block_index: Option<u64>) {
    record_event(&Event::CloseVault {
        vault_id,
        block_index,
    });
    state.close_vault(vault_id);
}

pub fn record_margin_transfer(state: &mut State, vault_id: u64, block_index: u64) {
    record_event(&Event::MarginTransfer {
        vault_id,
        block_index,
    });
    state.pending_margin_transfers.remove(&vault_id);
}

pub fn record_borrow_from_vault(
    state: &mut State,
    vault_id: u64,
    borrowed_amount: TAL,
    fee_amount: TAL,
    block_index: u64,
) {
    record_event(&Event::BorrowFromVault {
        vault_id,
        block_index,
        fee_amount,
        borrowed_amount,
    });
    state.borrow_from_vault(vault_id, borrowed_amount);
    state.provide_liquidity(fee_amount, state.developer_principal);
}

pub fn record_repayed_to_vault(
    state: &mut State,
    vault_id: u64,
    repayed_amount: TAL,
    block_index: u64,
) {
    record_event(&Event::RepayToVault {
        vault_id,
        block_index,
        repayed_amount,
    });
    state.repay_to_vault(vault_id, repayed_amount);
}

pub fn record_add_margin_to_vault(
    state: &mut State,
    vault_id: u64,
    margin_added: CKBTC,
    block_index: u64,
) {
    record_event(&Event::AddMarginToVault {
        vault_id,
        margin_added,
        block_index,
    });
    state.add_margin_to_vault(vault_id, margin_added);
}

pub fn record_redemption_on_vaults(
    state: &mut State,
    owner: Principal,
    tal_amount: TAL,
    fee_amount: TAL,
    current_btc_rate: UsdBtc,
    tal_block_index: u64,
) {
    record_event(&Event::RedemptionOnVaults {
        owner,
        current_btc_rate,
        tal_amount,
        fee_amount,
        tal_block_index,
    });
    state.provide_liquidity(fee_amount, state.developer_principal);
    state.redeem_on_vaults(tal_amount, current_btc_rate);
    let margin: CKBTC = tal_amount / current_btc_rate;
    state
        .pending_redemption_transfer
        .insert(tal_block_index, PendingMarginTransfer { owner, margin });
}

pub fn record_redemption_transfered(
    state: &mut State,
    tal_block_index: u64,
    ckbtc_block_index: u64,
) {
    record_event(&Event::RedemptionTransfered {
        tal_block_index,
        ckbtc_block_index,
    });
    state.pending_redemption_transfer.remove(&tal_block_index);
}
