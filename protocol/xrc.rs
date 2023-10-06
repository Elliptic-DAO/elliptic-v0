use crate::logs::TRACE_XRC;
use crate::numeric::UsdBtc;
use crate::state::{mutate_state, read_state};
use crate::Decimal;
use crate::Mode;
use ic_canister_log::log;
use ic_xrc_types::GetExchangeRateResult;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use std::time::Duration;

pub const FETCHING_BTC_RATE_INTERVAL: Duration = Duration::from_secs(60);

pub async fn fetch_btc_rate() {
    let _guard = match crate::guard::FetchXrcGuard::new() {
        Some(guard) => guard,
        None => return,
    };

    match crate::management::fetch_btc_price().await {
        Ok(call_result) => match call_result {
            GetExchangeRateResult::Ok(exchange_rate_result) => {
                let rate = Decimal::from_u64(exchange_rate_result.rate).unwrap()
                    / Decimal::from_u64(10_u64.pow(exchange_rate_result.metadata.decimals))
                        .unwrap();
                if rate < dec!(1000) {
                    log!(
                        TRACE_XRC,
                        "[FetchPrice] bug: btc rate is below 1000$ switching to read-only at timestamp: {}",
                        exchange_rate_result.timestamp
                    );
                    mutate_state(|s| s.mode = Mode::ReadOnly);
                };
                log!(
                    TRACE_XRC,
                    "[FetchPrice] fetched new btc rate: {rate} with timestamp: {}",
                    exchange_rate_result.timestamp
                );
                mutate_state(|s| match s.last_btc_timestamp {
                    Some(last_btc_timestamp) => {
                        if last_btc_timestamp < exchange_rate_result.timestamp * 1_000_000_000 {
                            s.last_btc_rate = Some(UsdBtc::from(rate));
                            s.last_btc_timestamp =
                                Some(exchange_rate_result.timestamp * 1_000_000_000);
                        }
                    }
                    None => {
                        s.last_btc_rate = Some(UsdBtc::from(rate));
                        s.last_btc_timestamp = Some(exchange_rate_result.timestamp * 1_000_000_000);
                    }
                });
            }
            GetExchangeRateResult::Err(error) => ic_canister_log::log!(
                TRACE_XRC,
                "[FetchPrice] failed to call XRC canister with error: {error:?}"
            ),
        },
        Err(error) => ic_canister_log::log!(
            TRACE_XRC,
            "[FetchPrice] failed to call XRC canister with error: {error}"
        ),
    }
    if let Some(last_btc_rate) = read_state(|s| s.last_btc_rate) {
        mutate_state(|s| s.update_total_collateral_ratio_and_mode(last_btc_rate));
    }
    if read_state(|s| s.mode != crate::Mode::ReadOnly) {
        crate::check_vaults();
    }
}
