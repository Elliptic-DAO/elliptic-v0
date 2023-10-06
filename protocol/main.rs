use candid::{candid_method, Principal};
use ic_canister_log::log;
use ic_canisters_http_types::{HttpRequest, HttpResponse, HttpResponseBuilder};
use ic_cdk_macros::{init, post_upgrade, query, update};
use protocol_canister::event::Event;
use protocol_canister::logs::INFO;
use protocol_canister::numeric::UsdBtc;
use protocol_canister::state::{read_state, replace_state, Mode, State};
use protocol_canister::storage::events;
use protocol_canister::vault::{CandidVault, OpenVaultSuccess, VaultArg};
use protocol_canister::{
    Fees, GetEventsArg, LiquidityStatus, ProtocolArg, ProtocolError, ProtocolStatus, SuccessWithFee,
};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[cfg(feature = "self_check")]
fn ok_or_die(result: Result<(), String>) {
    if let Err(msg) = result {
        ic_cdk::println!("{}", msg);
        ic_cdk::trap(&msg);
    }
}

/// Checks that Elliptic Core Canister state is internally consistent.
#[cfg(feature = "self_check")]
fn check_invariants() -> Result<(), String> {
    use protocol_canister::event::replay;

    read_state(|s| {
        s.check_invariants()?;

        let events: Vec<_> = protocol_canister::storage::events().collect();
        let recovered_state = replay(events.clone().into_iter())
            .unwrap_or_else(|e| panic!("failed to replay log {:?}: {:?}", events, e));

        recovered_state.check_invariants()?;

        // A running timer can temporarily violate invariants.
        if !s.is_timer_running {
            s.check_semantically_eq(&recovered_state)?;
        }

        Ok(())
    })
}

fn check_postcondition<T>(t: T) -> T {
    #[cfg(feature = "self_check")]
    ok_or_die(check_invariants());
    t
}

fn validate_call() -> Result<(), ProtocolError> {
    if ic_cdk::caller() == Principal::anonymous() {
        return Err(ProtocolError::AnonymousCallerNotAllowed);
    }
    read_state(|s| s.check_price_not_too_old())
}

fn validate_mode() -> Result<(), ProtocolError> {
    match read_state(|s| s.mode) {
        Mode::ReadOnly => {
            Err(ProtocolError::TemporarilyUnavailable(
                "protocol temporarly unavailable, please wait for an upgrade or for total collateral ratio to go above 100%".to_string(),
            ))
        }
        Mode::GeneralAvailability => Ok(()),
        Mode::Recovery => Ok(())
    }
}

fn setup_timers() {
    ic_cdk_timers::set_timer_interval(protocol_canister::xrc::FETCHING_BTC_RATE_INTERVAL, || {
        ic_cdk::spawn(protocol_canister::xrc::fetch_btc_rate())
    });
}

fn main() {}

#[candid_method(init)]
#[init]
fn init(arg: ProtocolArg) {
    match arg {
        ProtocolArg::Init(init_arg) => {
            log!(
                INFO,
                "[init] initialized elliptic protocol with args: {:?}",
                init_arg
            );
            protocol_canister::storage::record_event(&Event::Init(init_arg.clone()));
            replace_state(State::from(init_arg));
        }
        ProtocolArg::Upgrade(_) => ic_cdk::trap("expected Init got Upgrade"),
    }
    setup_timers();
}

#[post_upgrade]
fn post_upgrade(arg: ProtocolArg) {
    use protocol_canister::event::replay;
    use protocol_canister::storage::{count_events, events, record_event};

    let start = ic_cdk::api::instruction_counter();

    log!(INFO, "[upgrade]: replaying {} events", count_events());

    match arg {
        ProtocolArg::Init(_) => ic_cdk::trap("expected Upgrade got Init"),
        ProtocolArg::Upgrade(upgrade_args) => {
            log!(
                INFO,
                "[upgrade]: updating configuration with {:?}",
                upgrade_args
            );
            record_event(&Event::Upgrade(upgrade_args));
        }
    }

    let state = replay(events()).unwrap_or_else(|e| {
        ic_cdk::trap(&format!(
            "[upgrade]: failed to replay the event log: {:?}",
            e
        ))
    });

    replace_state(state);

    let end = ic_cdk::api::instruction_counter();

    log!(
        INFO,
        "[upgrade]: replaying events consumed {} instructions",
        end - start
    );

    setup_timers();
}

#[candid_method(query)]
#[query]
fn get_protocol_status() -> ProtocolStatus {
    read_state(|s| ProtocolStatus {
        last_btc_rate: s
            .last_btc_rate
            .unwrap_or(UsdBtc::from(Decimal::ZERO))
            .to_f64(),
        last_btc_timestamp: s.last_btc_timestamp.unwrap_or(0),
        total_ckbtc_margin: s.total_ckbtc_margin_amount().to_u64(),
        total_tal_borrowed: s.total_borrowed_tal_amount().to_u64(),
        total_collateral_ratio: s.total_collateral_ratio.to_f64(),
        mode: s.mode,
    })
}

#[candid_method(query)]
#[query]
fn get_fees(redeemed_amount: u64) -> Fees {
    read_state(|s| Fees {
        borrowing_fee: s.get_borrowing_fee().to_f64(),
        redemption_fee: s.get_redemption_fee(redeemed_amount.into()).to_f64(),
    })
}

#[candid_method(query)]
#[query]
fn get_vault_history(vault_id: u64) -> Vec<Event> {
    if ic_cdk::api::data_certificate().is_none() {
        ic_cdk::trap("update call rejected");
    }

    let mut vault_events: Vec<Event> = vec![];
    for event in events() {
        if event.is_vault_related(&vault_id) {
            vault_events.push(event);
        }
    }
    vault_events
}

#[candid_method(query)]
#[query]
fn get_events(args: GetEventsArg) -> Vec<Event> {
    if ic_cdk::api::data_certificate().is_none() {
        ic_cdk::trap("update call rejected");
    }
    const MAX_EVENTS_PER_QUERY: usize = 2000;

    events()
        .skip(args.start as usize)
        .take(MAX_EVENTS_PER_QUERY.min(args.length as usize))
        .collect()
}

#[candid_method(query)]
#[query]
fn get_liquidity_status(owner: Principal) -> LiquidityStatus {
    let total_liquidity_provided = read_state(|s| s.total_provided_liquidity_amount());
    let liquidity_pool_share = if total_liquidity_provided == 0 {
        0.0
    } else {
        read_state(|s| {
            (s.get_provided_liquidity(owner) / s.total_provided_liquidity_amount()).to_f64()
        })
    };
    read_state(|s| LiquidityStatus {
        liquidity_provided: s.get_provided_liquidity(owner).to_u64(),
        total_liquidity_provided: s.total_provided_liquidity_amount().to_u64(),
        liquidity_pool_share,
        available_liquidity_reward: s.get_liquidity_returns_of(owner).to_u64(),
        total_available_returns: s.total_available_returns().to_u64(),
    })
}

#[candid_method(query)]
#[query]
fn get_vaults(target: Option<Principal>) -> Vec<CandidVault> {
    match target {
        Some(target) => read_state(|s| match s.principal_to_vault_ids.get(&target) {
            Some(vault_ids) => vault_ids
                .iter()
                .map(|id| {
                    let vault = s.vault_id_to_vaults.get(id).cloned().unwrap();
                    CandidVault {
                        owner: vault.owner,
                        borrowed_tal_amount: vault.borrowed_tal_amount.to_u64(),
                        ckbtc_margin_amount: vault.ckbtc_margin_amount.to_u64(),
                        vault_id: vault.vault_id,
                    }
                })
                .collect(),
            None => vec![],
        }),
        None => read_state(|s| {
            s.vault_id_to_vaults
                .values()
                .map(|vault| CandidVault {
                    owner: vault.owner,
                    borrowed_tal_amount: vault.borrowed_tal_amount.to_u64(),
                    ckbtc_margin_amount: vault.ckbtc_margin_amount.to_u64(),
                    vault_id: vault.vault_id,
                })
                .collect::<Vec<CandidVault>>()
        }),
    }
}

// Vault related operations

#[candid_method(update)]
#[update]
async fn redeem_ckbtc(tal_amount: u64) -> Result<SuccessWithFee, ProtocolError> {
    validate_call()?;
    validate_mode()?;
    check_postcondition(protocol_canister::vault::redeem_ckbtc(tal_amount).await)
}

#[candid_method(update)]
#[update]
async fn open_vault(ckbtc_margin: u64) -> Result<OpenVaultSuccess, ProtocolError> {
    validate_call()?;
    check_postcondition(protocol_canister::vault::open_vault(ckbtc_margin).await)
}

#[candid_method(update)]
#[update]
async fn borrow_from_vault(arg: VaultArg) -> Result<SuccessWithFee, ProtocolError> {
    validate_call()?;
    validate_mode()?;
    check_postcondition(protocol_canister::vault::borrow_from_vault(arg).await)
}

#[candid_method(update)]
#[update]
async fn repay_to_vault(arg: VaultArg) -> Result<u64, ProtocolError> {
    validate_call()?;
    check_postcondition(protocol_canister::vault::repay_to_vault(arg).await)
}

#[candid_method(update)]
#[update]
async fn add_margin_to_vault(arg: VaultArg) -> Result<u64, ProtocolError> {
    validate_call()?;
    check_postcondition(protocol_canister::vault::add_margin_to_vault(arg).await)
}

#[candid_method(update)]
#[update]
async fn close_vault(vault_id: u64) -> Result<Option<u64>, ProtocolError> {
    validate_call()?;
    check_postcondition(protocol_canister::vault::close_vault(vault_id).await)
}

// Liquidity related operations

#[candid_method(update)]
#[update]
async fn provide_liquidity(amount: u64) -> Result<u64, ProtocolError> {
    validate_call()?;
    check_postcondition(protocol_canister::liquidity_pool::provide_liquidity(amount).await)
}

#[candid_method(update)]
#[update]
async fn withdraw_liquidity(amount: u64) -> Result<u64, ProtocolError> {
    validate_call()?;
    check_postcondition(protocol_canister::liquidity_pool::withdraw_liquidity(amount).await)
}

#[candid_method(update)]
#[update]
async fn claim_liquidity_returns() -> Result<u64, ProtocolError> {
    validate_call()?;
    check_postcondition(protocol_canister::liquidity_pool::claim_liquidity_returns().await)
}

#[query]
fn http_request(req: HttpRequest) -> HttpResponse {
    use ic_metrics_encoder::MetricsEncoder;
    if ic_cdk::api::data_certificate().is_none() {
        ic_cdk::trap("update call rejected");
    }

    if req.path() == "/metrics" {
        let mut writer = MetricsEncoder::new(vec![], ic_cdk::api::time() as i64 / 1_000_000);

        fn encode_metrics(w: &mut MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
            read_state(|s| {
                w.gauge_vec("cycle_balance", "Cycle balance of this canister.")?
                    .value(
                        &[("canister", "elliptic-protocol")],
                        ic_cdk::api::canister_balance128() as f64,
                    )?;

                w.encode_gauge(
                    "elliptic_active_vault_count",
                    s.vault_id_to_vaults.len() as f64,
                    "Count of active vaults in the system.",
                )?;

                w.encode_gauge(
                    "elliptic_vault_owners_count",
                    s.principal_to_vault_ids.keys().len() as f64,
                    "Count of owners of active vaults.",
                )?;

                w.encode_gauge(
                    "elliptic_total_provided_liquidity_amount",
                    s.total_provided_liquidity_amount().to_u64() as f64,
                    "Provided amount of liquidity.",
                )?;

                w.encode_gauge(
                    "elliptic_liquidity_providers_count",
                    s.liquidity_pool.len() as f64,
                    "Count of liquidity providers.",
                )?;

                w.encode_gauge(
                    "elliptic_pending_margin_transfer_count",
                    s.pending_margin_transfers.len() as f64,
                    "Pending margin transfers count.",
                )?;

                w.encode_gauge(
                    "elliptic_liquidity_providers_rewards",
                    s.total_available_returns().to_u64() as f64,
                    "Available rewards for liquidity providers.",
                )?;

                w.encode_gauge(
                    "elliptic_pending_margin_transfers_count",
                    s.pending_margin_transfers.len() as f64,
                    "Pending margin transfers count.",
                )?;

                w.encode_gauge(
                    "elliptic_pending_redemption_transfer_count",
                    s.pending_redemption_transfer.len() as f64,
                    "Pending redemption transfers count.",
                )?;

                w.encode_gauge(
                    "elliptic_btc_rate",
                    s.last_btc_rate.unwrap_or(UsdBtc::from(dec!(0))).to_f64(),
                    "BTC rate.",
                )?;

                let total_ckbtc_dec = Decimal::from_u64(s.total_ckbtc_margin_amount().0)
                    .expect("failed to construct decimal from u64")
                    / dec!(100_000_000);

                w.encode_gauge(
                    "elliptic_total_ckbtc_margin",
                    total_ckbtc_dec.to_f64().unwrap(),
                    "Total ckBTC Margin.",
                )?;

                w.encode_gauge(
                    "elliptic_total_tvl",
                    (total_ckbtc_dec * s.last_btc_rate.unwrap_or(UsdBtc::from(dec!(0))).0)
                        .to_f64()
                        .unwrap(),
                    "Total TVL.",
                )?;

                let total_borrowed_tal = Decimal::from_u64(s.total_borrowed_tal_amount().0)
                    .expect("failed to construct decimal from u64")
                    / dec!(100_000_000);

                w.encode_gauge(
                    "elliptic_total_borrowed_amount",
                    total_borrowed_tal.to_f64().unwrap(),
                    "Total borrowed TAL.",
                )?;

                w.encode_gauge(
                    "elliptic_total_collateral_ratio",
                    s.total_collateral_ratio.to_f64(),
                    "TCR.",
                )?;

                Ok(())
            })
        }

        match encode_metrics(&mut writer) {
            Ok(()) => HttpResponseBuilder::ok()
                .header("Content-Type", "text/plain; version=0.0.4")
                .with_body_and_content_length(writer.into_inner())
                .build(),
            Err(err) => {
                HttpResponseBuilder::server_error(format!("Failed to encode metrics: {}", err))
                    .build()
            }
        }
    } else if req.path() == "/logs" {
        use protocol_canister::logs::{Log, Priority};
        use serde_json;
        use std::str::FromStr;

        let max_skip_timestamp = match req.raw_query_param("time") {
            Some(arg) => match u64::from_str(arg) {
                Ok(value) => value,
                Err(_) => {
                    return HttpResponseBuilder::bad_request()
                        .with_body_and_content_length("failed to parse the 'time' parameter")
                        .build()
                }
            },
            None => 0,
        };

        let mut entries: Log = Default::default();

        match req.raw_query_param("priority") {
            Some(priority_str) => match Priority::from_str(priority_str) {
                Ok(priority) => match priority {
                    Priority::Info => entries.push_logs(Priority::Info),
                    Priority::TraceXrc => entries.push_logs(Priority::TraceXrc),
                    Priority::Debug => entries.push_logs(Priority::Debug),
                },
                Err(_) => entries.push_all(),
            },
            None => entries.push_all(),
        }

        entries
            .entries
            .retain(|entry| entry.timestamp >= max_skip_timestamp);
        let mut entries_bytes: Vec<u8> = serde_json::to_string(&entries)
            .unwrap_or_default()
            .into_bytes();

        // Truncate bytes to avoid having more than 2MB response.
        let max_size_bytes: usize = 1_900_000;
        entries_bytes.truncate(max_size_bytes);

        HttpResponseBuilder::ok()
            .header("Content-Type", "application/json; charset=utf-8")
            .with_body_and_content_length(entries_bytes)
            .build()
    } else if req.path() == "/dashboard" {
        use protocol_canister::dashboard::build_dashboard;

        let dashboard = build_dashboard();
        HttpResponseBuilder::ok()
            .header("Content-Type", "text/html; charset=utf-8")
            .with_body_and_content_length(dashboard)
            .build()
    } else {
        HttpResponseBuilder::not_found().build()
    }
}

// Checks the real candid interface against the one declared in the did file
#[test]
fn check_candid_interface_compatibility() {
    fn source_to_str(source: &candid::utils::CandidSource) -> String {
        match source {
            candid::utils::CandidSource::File(f) => {
                std::fs::read_to_string(f).unwrap_or_else(|_| "".to_string())
            }
            candid::utils::CandidSource::Text(t) => t.to_string(),
        }
    }

    fn check_service_compatible(
        new_name: &str,
        new: candid::utils::CandidSource,
        old_name: &str,
        old: candid::utils::CandidSource,
    ) {
        let new_str = source_to_str(&new);
        let old_str = source_to_str(&old);
        match candid::utils::service_equal(new, old) {
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "{} is not compatible with {}!\n\n\
            {}:\n\
            {}\n\n\
            {}:\n\
            {}\n",
                    new_name, old_name, new_name, new_str, old_name, old_str
                );
                panic!("{:?}", e);
            }
        }
    }

    candid::export_service!();

    let new_interface = __export_service();

    // check the public interface against the actual one
    let old_interface =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("protocol.did");

    check_service_compatible(
        "actual ledger candid interface",
        candid::utils::CandidSource::Text(&new_interface),
        "declared candid interface in protocol.did file",
        candid::utils::CandidSource::File(old_interface.as_path()),
    );
}
