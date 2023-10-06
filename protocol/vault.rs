use crate::event::{
    record_add_margin_to_vault, record_borrow_from_vault, record_open_vault,
    record_redemption_on_vaults, record_repayed_to_vault,
};
use crate::guard::GuardPrincipal;
use crate::logs::{DEBUG, INFO};
use crate::management::{mint_tal, transfer_ckbtc_from, transfer_tal_from};
use crate::numeric::{CKBTC, TAL};
use crate::{
    mutate_state, read_state, ProtocolError, SuccessWithFee, MIN_CKBTC_AMOUNT, MIN_TAL_AMOUNT,
};
use candid::{CandidType, Deserialize, Principal};
use ic_canister_log::log;
use icrc_ledger_types::icrc2::transfer_from::TransferFromError;
use serde::Serialize;
use std::time::Duration;

#[derive(CandidType, Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OpenVaultSuccess {
    pub vault_id: u64,
    pub block_index: u64,
}

#[derive(CandidType, Deserialize)]
pub struct VaultArg {
    pub vault_id: u64,
    pub amount: u64,
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize, Serialize, PartialOrd, Ord)]
pub struct Vault {
    pub owner: Principal,
    pub borrowed_tal_amount: TAL,
    pub ckbtc_margin_amount: CKBTC,
    pub vault_id: u64,
}

#[derive(CandidType, Serialize, Deserialize, Debug)]
pub struct CandidVault {
    pub owner: Principal,
    pub borrowed_tal_amount: u64,
    pub ckbtc_margin_amount: u64,
    pub vault_id: u64,
}

pub async fn redeem_ckbtc(_tal_amount: u64) -> Result<SuccessWithFee, ProtocolError> {
    let caller = ic_cdk::api::caller();
    let _guard_principal = GuardPrincipal::new(caller)?;

    let tal_amount: TAL = _tal_amount.into();

    if tal_amount < MIN_TAL_AMOUNT {
        return Err(ProtocolError::AmountTooLow {
            minimum_amount: MIN_TAL_AMOUNT.to_u64(),
        });
    }

    let current_btc_rate = read_state(|s| s.last_btc_rate.expect("no btc rate entry"));

    match transfer_tal_from(tal_amount, caller).await {
        Ok(block_index) => {
            let fee_amount = mutate_state(|s| {
                let base_fee = s.get_redemption_fee(tal_amount);
                s.current_base_rate = base_fee;
                s.last_redemption_time = ic_cdk::api::time();
                let fee_amount = tal_amount * base_fee;

                record_redemption_on_vaults(
                    s,
                    caller,
                    tal_amount - fee_amount,
                    fee_amount,
                    current_btc_rate,
                    block_index,
                );
                fee_amount
            });
            ic_cdk_timers::set_timer(std::time::Duration::from_secs(0), || {
                ic_cdk::spawn(crate::process_pending_transfer())
            });
            Ok(SuccessWithFee {
                block_index,
                fee_amount_paid: fee_amount.to_u64(),
            })
        }
        Err(transfer_from_error) => Err(ProtocolError::TransferFromError(
            transfer_from_error,
            tal_amount.to_u64(),
        )),
    }
}

pub async fn open_vault(ckbtc_margin: u64) -> Result<OpenVaultSuccess, ProtocolError> {
    let caller = ic_cdk::api::caller();
    let _guard_principal = GuardPrincipal::new(caller)?;

    let ckbtc_margin_amount = ckbtc_margin.into();

    if ckbtc_margin_amount < MIN_CKBTC_AMOUNT {
        return Err(ProtocolError::AmountTooLow {
            minimum_amount: MIN_CKBTC_AMOUNT.to_u64(),
        });
    }

    match transfer_ckbtc_from(ckbtc_margin_amount, caller).await {
        Ok(block_index) => {
            let vault_id = mutate_state(|s| {
                let vault_id = s.increment_vault_id();
                record_open_vault(
                    s,
                    Vault {
                        owner: caller,
                        borrowed_tal_amount: 0.into(),
                        ckbtc_margin_amount,
                        vault_id,
                    },
                    block_index,
                );
                vault_id
            });
            log!(INFO, "[open_vault] opened vault with id: {vault_id}");
            Ok(OpenVaultSuccess {
                vault_id,
                block_index,
            })
        }
        Err(transfer_from_error) => {
            if let TransferFromError::BadFee { expected_fee } = transfer_from_error.clone() {
                mutate_state(|s| {
                    let expected_fee: u64 = expected_fee
                        .0
                        .try_into()
                        .expect("failed to convert Nat to u64");
                    s.ckbtc_ledger_fee = CKBTC::from(expected_fee);
                });
            };
            Err(ProtocolError::TransferFromError(
                transfer_from_error,
                ckbtc_margin_amount.to_u64(),
            ))
        }
    }
}

pub async fn borrow_from_vault(arg: VaultArg) -> Result<SuccessWithFee, ProtocolError> {
    let caller = ic_cdk::api::caller();
    let _guard_principal = GuardPrincipal::new(caller)?;

    let amount: TAL = arg.amount.into();

    if amount < MIN_TAL_AMOUNT {
        return Err(ProtocolError::AmountTooLow {
            minimum_amount: MIN_TAL_AMOUNT.to_u64(),
        });
    }

    let (vault_id, amount) = (arg.vault_id, amount);

    let (vault, last_btc_rate) = read_state(|s| {
        (
            s.vault_id_to_vaults.get(&vault_id).cloned().unwrap(),
            s.last_btc_rate.expect("no btc rate"),
        )
    });

    if caller != vault.owner {
        return Err(ProtocolError::CallerNotOwner);
    }

    let max_borrowable_amount = vault.ckbtc_margin_amount * last_btc_rate
        / read_state(|s| s.mode.get_minimum_liquidation_collateral_ratio());

    if vault.borrowed_tal_amount + amount > max_borrowable_amount {
        return Err(ProtocolError::GenericError(format!("failed to borrow from vault, max borrowable amount: {max_borrowable_amount}, already borrowed: {}, asked to borrow {amount} \n last_btc_rate: {last_btc_rate}", vault.borrowed_tal_amount)));
    }

    let fee: TAL = read_state(|s| amount * s.get_borrowing_fee());

    match mint_tal(amount - fee, caller).await {
        Ok(block_index) => {
            log!(DEBUG, "[borrow_from_vault] {caller} borrowed {amount}, from vault {vault_id} with a fee of {fee} at block {block_index}");
            mutate_state(|s| {
                record_borrow_from_vault(s, vault_id, amount, fee, block_index);
            });
            Ok(SuccessWithFee {
                block_index,
                fee_amount_paid: fee.to_u64(),
            })
        }
        Err(mint_error) => Err(ProtocolError::TransferError(mint_error)),
    }
}

pub async fn repay_to_vault(arg: VaultArg) -> Result<u64, ProtocolError> {
    let caller = ic_cdk::api::caller();
    let _guard_principal = GuardPrincipal::new(caller)?;

    let vault = read_state(|s| s.vault_id_to_vaults.get(&arg.vault_id).cloned().unwrap());
    let amount: TAL = arg.amount.into();

    if caller != vault.owner {
        return Err(ProtocolError::CallerNotOwner);
    }

    if amount < MIN_TAL_AMOUNT {
        return Err(ProtocolError::AmountTooLow {
            minimum_amount: MIN_TAL_AMOUNT.to_u64(),
        });
    }

    if vault.borrowed_tal_amount < amount {
        return Err(ProtocolError::GenericError(format!(
            "cannot repay more than borrowed, borrowed: {} TAL, asked to repay: {} TAL",
            (vault.borrowed_tal_amount),
            (arg.amount)
        )));
    }

    match transfer_tal_from(amount, caller).await {
        Ok(block_index) => {
            log!(
                DEBUG,
                "[repay_to_vault] {caller} borrowed {}, from vault {} at block {block_index}",
                arg.amount,
                arg.vault_id
            );
            mutate_state(|s| record_repayed_to_vault(s, arg.vault_id, amount, block_index));
            Ok(block_index)
        }
        Err(transfer_from_error) => Err(ProtocolError::TransferFromError(
            transfer_from_error,
            arg.amount,
        )),
    }
}

pub async fn add_margin_to_vault(arg: VaultArg) -> Result<u64, ProtocolError> {
    let caller = ic_cdk::api::caller();
    let _guard_principal = GuardPrincipal::new(caller)?;

    let amount = arg.amount.into();

    if amount < MIN_CKBTC_AMOUNT {
        return Err(ProtocolError::AmountTooLow {
            minimum_amount: MIN_CKBTC_AMOUNT.to_u64(),
        });
    }

    let vault = read_state(|s| s.vault_id_to_vaults.get(&arg.vault_id).cloned().unwrap());

    if caller != vault.owner {
        return Err(ProtocolError::CallerNotOwner);
    }

    match transfer_ckbtc_from(amount, caller).await {
        Ok(block_index) => {
            log!(
                DEBUG,
                "[add_margin_to_vault] {caller} added margin {} to vault {} at block {block_index}",
                amount,
                arg.vault_id
            );
            mutate_state(|s| record_add_margin_to_vault(s, arg.vault_id, amount, block_index));
            Ok(block_index)
        }
        Err(error) => {
            if let TransferFromError::BadFee { expected_fee } = error.clone() {
                mutate_state(|s| {
                    let expected_fee: u64 = expected_fee
                        .0
                        .try_into()
                        .expect("failed to convert Nat to u64");
                    s.ckbtc_ledger_fee = CKBTC::from(expected_fee);
                });
            };
            Err(ProtocolError::TransferFromError(error, amount.to_u64()))
        }
    }
}

pub async fn close_vault(vault_id: u64) -> Result<Option<u64>, ProtocolError> {
    let caller = ic_cdk::api::caller();
    let _guard_principal = GuardPrincipal::new(caller)?;

    let vault = read_state(|s| s.vault_id_to_vaults.get(&vault_id).cloned().unwrap());
    if caller != vault.owner {
        return Err(ProtocolError::CallerNotOwner);
    }

    let amount_to_pay_off = read_state(|s| match s.vault_id_to_vaults.get(&vault_id) {
        Some(vault) => vault.borrowed_tal_amount,
        None => panic!("vault not found"),
    });
    if amount_to_pay_off == 0 {
        mutate_state(|s| {
            crate::event::record_close_vault(s, vault_id, None);
        });
        ic_cdk_timers::set_timer(Duration::from_secs(0), || {
            ic_cdk::spawn(crate::process_pending_transfer())
        });
        return Ok(None);
    }
    match transfer_tal_from(amount_to_pay_off, caller).await {
        Ok(block_index) => {
            log!(
                DEBUG,
                "[close_vault] closed vault {vault_id} at block {block_index}"
            );
            mutate_state(|s| {
                crate::event::record_close_vault(s, vault_id, Some(block_index));
            });
            ic_cdk_timers::set_timer(Duration::from_secs(0), || {
                ic_cdk::spawn(crate::process_pending_transfer())
            });
            Ok(Some(block_index))
        }
        Err(burn_from_error) => Err(ProtocolError::TransferFromError(
            burn_from_error,
            amount_to_pay_off.to_u64(),
        )),
    }
}
