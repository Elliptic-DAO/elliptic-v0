use crate::numeric::{CKBTC, TAL};
use crate::state::read_state;
use candid::{Nat, Principal};
use ic_icrc1_client_cdk::{CdkRuntime, ICRC1Client};
use ic_xrc_types::{Asset, AssetClass, GetExchangeRateRequest, GetExchangeRateResult};
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc1::transfer::{TransferArg, TransferError};
use icrc_ledger_types::icrc2::transfer_from::{TransferFromArgs, TransferFromError};
use std::fmt;

/// Represents an error from a management canister call
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallError {
    method: String,
    reason: Reason,
}

impl CallError {
    /// Returns the name of the method that resulted in this error.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Returns the failure reason.
    pub fn reason(&self) -> &Reason {
        &self.reason
    }
}

impl fmt::Display for CallError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "management call '{}' failed: {}",
            self.method, self.reason
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// The reason for the management call failure.
pub enum Reason {
    /// Failed to send a signature request because the local output queue is
    /// full.
    QueueIsFull,
    /// The canister does not have enough cycles to submit the request.
    OutOfCycles,
    /// The call failed with an error.
    CanisterError(String),
    /// The management canister rejected the signature request (not enough
    /// cycles, the ECDSA subnet is overloaded, etc.).
    Rejected(String),
}

impl fmt::Display for Reason {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueIsFull => write!(fmt, "the canister queue is full"),
            Self::OutOfCycles => write!(fmt, "the canister is out of cycles"),
            Self::CanisterError(msg) => write!(fmt, "canister error: {}", msg),
            Self::Rejected(msg) => {
                write!(fmt, "the management canister rejected the call: {}", msg)
            }
        }
    }
}

/// Query the XRC canister to retrieve the last BTC/USD price.
/// https://github.com/dfinity/exchange-rate-canister
pub async fn fetch_btc_price() -> Result<GetExchangeRateResult, String> {
    const XRC_CALL_COST_CYCLES: u64 = 10_000_000_000;
    const XRC_MARGIN_SEC: u64 = 60;

    let btc = Asset {
        symbol: "BTC".to_string(),
        class: AssetClass::Cryptocurrency,
    };
    let usd = Asset {
        symbol: "USD".to_string(),
        class: AssetClass::FiatCurrency,
    };

    // Take few minutes back to be sure to have data.
    let timestamp_sec = ic_cdk::api::time() / crate::SEC_NANOS - XRC_MARGIN_SEC;

    // Retrieve last BTC/USD value.
    let args = GetExchangeRateRequest {
        base_asset: btc,
        quote_asset: usd,
        timestamp: Some(timestamp_sec),
    };

    let xrc_principal = read_state(|s| s.xrc_principal);

    let res_xrc: Result<(GetExchangeRateResult,), (i32, String)> =
        ic_cdk::api::call::call_with_payment(
            xrc_principal,
            "get_exchange_rate",
            (args,),
            XRC_CALL_COST_CYCLES,
        )
        .await
        .map_err(|(code, msg)| (code as i32, msg));
    match res_xrc {
        Ok((xr,)) => Ok(xr),
        Err((code, msg)) => Err(format!(
            "Error while calling XRC canister ({}): {:?}",
            code, msg
        )),
    }
}

pub async fn mint_tal(amount: TAL, to: Principal) -> Result<u64, TransferError> {
    let client = ICRC1Client {
        runtime: CdkRuntime,
        ledger_canister_id: crate::state::read_state(|s| s.taler_ledger_principal),
    };
    let block_index = client
        .transfer(TransferArg {
            from_subaccount: None,
            to: Account {
                owner: to,
                subaccount: None,
            },
            fee: None,
            created_at_time: None,
            memo: None,
            amount: amount.to_nat(),
        })
        .await
        .map_err(|e| TransferError::GenericError {
            error_code: (Nat::from(e.0)),
            message: (e.1),
        })??;
    Ok(block_index)
}

pub async fn transfer_tal_from(amount: TAL, caller: Principal) -> Result<u64, TransferFromError> {
    let client = ICRC1Client {
        runtime: CdkRuntime,
        ledger_canister_id: read_state(|s| s.taler_ledger_principal),
    };
    let protocol_id = ic_cdk::id();
    let block_index = client
        .transfer_from(TransferFromArgs {
            spender_subaccount: None,
            from: Account {
                owner: caller,
                subaccount: None,
            },
            to: Account {
                owner: protocol_id,
                subaccount: None,
            },
            amount: amount.to_nat(),
            fee: None,
            created_at_time: None,
            memo: None,
        })
        .await
        .map_err(|e| TransferFromError::GenericError {
            error_code: (Nat::from(e.0)),
            message: (e.1),
        })??;
    Ok(block_index)
}

pub async fn transfer_ckbtc_from(
    amount: CKBTC,
    caller: Principal,
) -> Result<u64, TransferFromError> {
    let client = ICRC1Client {
        runtime: CdkRuntime,
        ledger_canister_id: read_state(|s| s.ckbtc_ledger_principal),
    };
    let protocol_id = ic_cdk::id();
    let ckbtc_transfer_fee = read_state(|s| s.ckbtc_ledger_fee);
    let block_index = client
        .transfer_from(TransferFromArgs {
            spender_subaccount: None,
            from: Account {
                owner: caller,
                subaccount: None,
            },
            to: Account {
                owner: protocol_id,
                subaccount: None,
            },
            amount: amount.to_nat(),
            fee: Some(ckbtc_transfer_fee.to_nat()),
            created_at_time: None,
            memo: None,
        })
        .await
        .map_err(|e| TransferFromError::GenericError {
            error_code: (Nat::from(e.0)),
            message: (e.1),
        })??;
    Ok(block_index)
}

pub async fn transfer_ckbtc(amount: CKBTC, to: Principal) -> Result<u64, TransferError> {
    let client = ICRC1Client {
        runtime: CdkRuntime,
        ledger_canister_id: crate::state::read_state(|s| s.ckbtc_ledger_principal),
    };
    let ckbtc_transfer_fee = read_state(|s| s.ckbtc_ledger_fee);
    let block_index = client
        .transfer(TransferArg {
            from_subaccount: None,
            to: Account {
                owner: to,
                subaccount: None,
            },
            fee: Some(ckbtc_transfer_fee.to_nat()),
            created_at_time: None,
            memo: None,
            amount: amount.to_nat(),
        })
        .await
        .map_err(|e| TransferError::GenericError {
            error_code: (Nat::from(e.0)),
            message: (e.1),
        })??;
    Ok(block_index)
}
