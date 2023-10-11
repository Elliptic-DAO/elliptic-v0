use crate::event::Event;
use crate::logs::Log;
use crate::numeric::{Ratio, UsdBtc, CKBTC, TAL};
use crate::state::{Mode, CKBTC_TRANSFER_FEE};
use crate::vault::{CandidVault, OpenVaultSuccess, VaultArg};
use crate::{
    Fees, InitArg, LiquidityStatus, ProtocolArg, ProtocolError, ProtocolStatus, SuccessWithFee,
    UpgradeArg,
};
use assert_matches::assert_matches;
use candid::{Decode, Encode, Nat, Principal};
use ic_base_types::{CanisterId, PrincipalId};
use ic_canisters_http_types::{HttpRequest, HttpResponse};
use ic_ic00_types::CanisterInstallMode;
use ic_icrc1_ledger::{FeatureFlags, InitArgs, LedgerArgument, UpgradeArgs as LedgerUpgradeArgs};
use ic_ledger_canister_core::archive::ArchiveOptions;
use ic_state_machine_tests::{StateMachine, WasmResult};
use ic_xrc_types::{Asset, AssetClass, GetExchangeRateRequest, GetExchangeRateResult};
use icrc_ledger_types::icrc1::account::Account;
use icrc_ledger_types::icrc1::transfer::{TransferArg, TransferError};
use icrc_ledger_types::icrc2::approve::{ApproveArgs, ApproveError};
use lazy_static::lazy_static;
use rand::RngCore;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::process::Command;
use xrc_mock::{ExchangeRate as ExchangeRateMock, Response, XrcMockInitPayload};

lazy_static! {
    static ref CARGO_BUILD_RESULT: Result<(), std::io::Error> = cargo_build();
}

const TAL_TRANSFER_FEE: TAL = TAL::new(1_000_000);
const INITIAL_BTC_RATE: UsdBtc = UsdBtc::new(dec!(20_000.0));
const LIQUIDATION_COLLATERAL_RATIO: Ratio = Ratio::new(dec!(1.1));
const ONE_CKBTC: CKBTC = CKBTC::new(100_000_000);
const RECOVERY_COLLATERAL_RATIO: Ratio = Ratio::new(dec!(1.5));
const TEN_E8S: u64 = 1_000_000_000;
const FIVE_E8S: u64 = 500_000_000;
const E8S: u64 = 100_000_000;
const ARCHIVE_TRIGGER_THRESHOLD: u64 = 2000;
const NUM_BLOCKS_TO_ARCHIVE: u64 = 1000;
const CYCLES_FOR_ARCHIVE_CREATION: u64 = 100_000_000_000_000;
const MAX_MESSAGE_SIZE_BYTES: u64 = 3_221_225_472;

fn cargo_build() -> Result<(), std::io::Error> {
    Command::new("cargo")
        .args(&[
            "build",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "-p",
            "protocol-canister",
            "--locked",
            "--features=self_check",
        ])
        .spawn()?
        .wait()?;
    Ok(())
}

fn xrc_wasm() -> Vec<u8> {
    let current_dir = std::env::current_dir().unwrap();
    let file_path =
        current_dir.join("../ic/bazel-bin/rs/rosetta-api/tvl/xrc_mock/xrc_mock_canister.wasm");
    std::fs::read(file_path).unwrap()
}

fn icrc1_ledger_wasm() -> Vec<u8> {
    let current_dir = std::env::current_dir().unwrap();
    let file_path =
        current_dir.join("../ic/bazel-bin/rs/rosetta-api/icrc1/ledger/ledger_canister.wasm");
    std::fs::read(file_path).unwrap()
}

fn protocol_wasm() -> Vec<u8> {
    let _ = *CARGO_BUILD_RESULT;
    let current_dir = std::env::current_dir().unwrap();
    let file_path =
        current_dir.join("./target/wasm32-unknown-unknown/release/protocol-canister.wasm");
    std::fs::read(file_path).unwrap()
}

fn assert_reply(result: WasmResult) -> Vec<u8> {
    match result {
        WasmResult::Reply(bytes) => bytes,
        WasmResult::Reject(reject) => {
            panic!("Expected a successful reply, got a reject: {}", reject)
        }
    }
}

fn install_taler_ledger(
    env: &StateMachine,
    icrc1_ledger_wasm: Vec<u8>,
    taler_ledger_id: CanisterId,
    core_id: CanisterId,
) {
    let init_args = InitArgs {
        minting_account: Account {
            owner: core_id.into(),
            subaccount: None,
        },
        fee_collector_account: None,
        initial_balances: vec![],
        transfer_fee: TAL_TRANSFER_FEE.to_nat(),
        token_name: "Taler".into(),
        accounts_overflow_trim_quantity: None,
        token_symbol: "TAL".into(),
        maximum_number_of_accounts: None,
        metadata: vec![],
        decimals: Some(2),
        archive_options: ArchiveOptions {
            trigger_threshold: ARCHIVE_TRIGGER_THRESHOLD as usize,
            num_blocks_to_archive: NUM_BLOCKS_TO_ARCHIVE as usize,
            node_max_memory_size_bytes: None,
            max_message_size_bytes: Some(MAX_MESSAGE_SIZE_BYTES),
            controller_id: PrincipalId::new_user_test_id(100),
            cycles_for_archive_creation: Some(CYCLES_FOR_ARCHIVE_CREATION),
            max_transactions_per_response: None,
        },
        feature_flags: Some(FeatureFlags { icrc2: true }),
        max_memo_length: None,
    };
    let ledger_arg = LedgerArgument::Init(init_args);
    let args = Encode!(&ledger_arg).unwrap();
    env.install_wasm_in_mode(
        taler_ledger_id,
        CanisterInstallMode::Install,
        icrc1_ledger_wasm,
        args,
    )
    .unwrap();
}

fn install_core_canister(
    env: &StateMachine,
    core_canister_wasm: Vec<u8>,
    init_args: InitArg,
) -> CanisterId {
    let core_args = ProtocolArg::Init(init_args);
    let args = Encode!(&core_args).unwrap();
    env.install_canister_with_cycles(core_canister_wasm, args, None, u128::MAX.into())
        .unwrap()
}

fn install_ckbtc_ledger(
    env: &StateMachine,
    icrc1_ledger_wasm: Vec<u8>,
    ckbtc_ledger_id: CanisterId,
    initial_balances: Vec<(Account, Nat)>,
) {
    let init_args = InitArgs {
        minting_account: Account {
            owner: PrincipalId::new(0, [0u8; 29]).0,
            subaccount: None,
        },
        fee_collector_account: None,
        accounts_overflow_trim_quantity: None,
        maximum_number_of_accounts: None,
        decimals: None,
        initial_balances,
        transfer_fee: CKBTC_TRANSFER_FEE.to_nat(),
        token_name: "ckBTC".into(),
        token_symbol: "ckBTC".into(),
        metadata: vec![],
        archive_options: ArchiveOptions {
            trigger_threshold: ARCHIVE_TRIGGER_THRESHOLD as usize,
            num_blocks_to_archive: NUM_BLOCKS_TO_ARCHIVE as usize,
            node_max_memory_size_bytes: None,
            max_message_size_bytes: Some(MAX_MESSAGE_SIZE_BYTES),
            controller_id: PrincipalId::new_user_test_id(100),
            cycles_for_archive_creation: Some(CYCLES_FOR_ARCHIVE_CREATION),
            max_transactions_per_response: None,
        },
        feature_flags: Some(FeatureFlags { icrc2: true }),
        max_memo_length: Some(80),
    };
    let ledger_arg = LedgerArgument::Init(init_args);
    let args = Encode!(&ledger_arg).unwrap();
    env.install_wasm_in_mode(
        ckbtc_ledger_id,
        CanisterInstallMode::Install,
        icrc1_ledger_wasm,
        args,
    )
    .unwrap();
}

struct EllipticSetup {
    pub env: StateMachine,
    pub principals: Vec<Principal>,
    pub taler_ledger_id: CanisterId,
    pub xrc_id: CanisterId,
    pub ckbtc_ledger_id: CanisterId,
    pub protocol_id: CanisterId,
}

pub fn get_users() -> Vec<Principal> {
    let mut rng = rand::thread_rng();
    let mut users = vec![];
    for _ in 0..10 {
        let mut buf = [0u8; 32];
        rng.fill_bytes(&mut buf);
        let principal = PrincipalId::new_self_authenticating(&buf).0;
        users.push(principal);
    }
    users
}

impl EllipticSetup {
    pub fn new() -> Self {
        let env = StateMachine::new();
        let xrc_args = XrcMockInitPayload {
            response: Response::ExchangeRate(ExchangeRateMock {
                base_asset: Some(Asset {
                    symbol: "BTC".into(),
                    class: AssetClass::Cryptocurrency,
                }),
                quote_asset: Some(Asset {
                    symbol: "USD".into(),
                    class: AssetClass::FiatCurrency,
                }),
                metadata: None,
                rate: INITIAL_BTC_RATE.to_e8s(),
            }),
        };
        let principals = get_users();
        let mut initial_balances: Vec<(Account, Nat)> = vec![];
        for principal in &principals {
            initial_balances.push((
                Account {
                    owner: *principal,
                    subaccount: None,
                },
                TEN_E8S.into(),
            ));
        }

        let xrc_args = Encode!(&xrc_args).unwrap();
        let xrc_id = env.install_canister(xrc_wasm(), xrc_args, None).unwrap();

        let taler_ledger_id = env.create_canister(None);
        let ckbtc_ledger_id = env.create_canister(None);

        install_ckbtc_ledger(&env, icrc1_ledger_wasm(), ckbtc_ledger_id, initial_balances);

        let init_args = InitArg {
            xrc_principal: xrc_id.into(),
            taler_ledger_principal: taler_ledger_id.into(),
            ckbtc_ledger_principal: ckbtc_ledger_id.into(),
            fee_e8s: 0,
            developer_principal: Principal::anonymous(),
        };

        let protocol_id = install_core_canister(&env, protocol_wasm(), init_args);

        install_taler_ledger(&env, icrc1_ledger_wasm(), taler_ledger_id, protocol_id);

        env.tick();
        env.advance_time(std::time::Duration::from_secs(60));
        env.tick();
        Self {
            env,
            principals,
            taler_ledger_id,
            ckbtc_ledger_id,
            xrc_id,
            protocol_id,
        }
    }

    fn with_fee(&mut self, fee_e8s: u64) {
        let init_args = InitArg {
            xrc_principal: self.xrc_id.into(),
            taler_ledger_principal: self.taler_ledger_id.into(),
            ckbtc_ledger_principal: self.ckbtc_ledger_id.into(),
            fee_e8s,
            developer_principal: Principal::anonymous(),
        };

        self.env
            .reinstall_canister(
                self.protocol_id,
                protocol_wasm(),
                Encode!(&ProtocolArg::Init(init_args)).unwrap(),
            )
            .expect("failed to reinstall the protocol canister");
        self.env.tick();
        self.env.advance_time(std::time::Duration::from_secs(60));
        self.env.tick();
    }

    pub fn set_btc_price(&self, rate: UsdBtc) {
        let xrc_args = XrcMockInitPayload {
            response: Response::ExchangeRate(ExchangeRateMock {
                base_asset: Some(Asset {
                    symbol: "BTC".into(),
                    class: AssetClass::Cryptocurrency,
                }),
                quote_asset: Some(Asset {
                    symbol: "USD".into(),
                    class: AssetClass::FiatCurrency,
                }),
                metadata: None,
                rate: rate.to_e8s(),
            }),
        };
        self.env
            .reinstall_canister(self.xrc_id, xrc_wasm(), Encode!(&xrc_args).unwrap())
            .expect("failed to upgrade the xrc canister");
        self.advance_time_and_tick(60);
    }

    pub fn advance_time_and_tick(&self, seconds: u64) {
        self.env
            .advance_time(std::time::Duration::from_secs(seconds));
        self.env.tick();
    }

    pub fn approve_ckbtc_and_open_vault(
        &self,
        called_by: Principal,
        margin: u64,
    ) -> Result<OpenVaultSuccess, ProtocolError> {
        assert_matches!(
            Decode!(
                &assert_reply(self.env.execute_ingress_as(
                    PrincipalId(called_by),
                    self.ckbtc_ledger_id,
                    "icrc2_approve",
                    Encode!(&ApproveArgs {
                        from_subaccount: None,
                        spender: Account { owner: self.protocol_id.into(), subaccount: None},
                        amount: u64::MAX.into(),
                        expected_allowance: None,
                        expires_at: None,
                        fee: None,
                        memo: None,
                        created_at_time: None,
                    }).unwrap()
                )
                .expect("failed to approve protocol canister")),
                Result<Nat, ApproveError>
            )
            .expect("failed to decode open_vault response"),
            Ok(_)
        );
        self.open_vault(called_by, margin)
    }

    pub fn open_vault(
        &self,
        called_by: Principal,
        margin: u64,
    ) -> Result<OpenVaultSuccess, ProtocolError> {
        Decode!(
            &assert_reply(self.env.execute_ingress_as(
                PrincipalId(called_by),
                self.protocol_id,
                "open_vault",
                Encode!(&margin).unwrap()
            )
            .expect("failed to open vault")),
            Result<OpenVaultSuccess, ProtocolError>
        )
        .expect("failed to decode open_vault response")
    }

    pub fn approve_elliptic_and_close_vault(
        &self,
        called_by: Principal,
        vault_id: u64,
    ) -> Result<Option<u64>, ProtocolError> {
        self.tal_approve_elliptic(called_by);
        self.close_vault(called_by, vault_id)
    }

    pub fn close_vault(
        &self,
        called_by: Principal,
        vault_id: u64,
    ) -> Result<Option<u64>, ProtocolError> {
        Decode!(
            &assert_reply(
                self.env
                    .execute_ingress_as(
                        PrincipalId(called_by),
                        self.protocol_id,
                        "close_vault",
                        Encode!(&vault_id).unwrap()
                    )
                    .expect("failed to open vault")
            ),
            Result<Option<u64>, ProtocolError>
        )
        .expect("failed to decode open_vault response")
    }

    pub fn tal_approve_elliptic(&self, called_by: Principal) {
        assert_matches!(
            Decode!(
                &assert_reply(self.env.execute_ingress_as(
                    PrincipalId(called_by),
                    self.taler_ledger_id,
                    "icrc2_approve",
                    Encode!(&ApproveArgs {
                        from_subaccount: None,
                        spender: Account { owner: self.protocol_id.into(), subaccount: None},
                        amount: u64::MAX.into(),
                        expected_allowance: None,
                        expires_at: None,
                        fee: None,
                        memo: None,
                        created_at_time: None,
                    }).unwrap()
                )
                .expect("failed to approve protocol canister")),
                Result<Nat, ApproveError>
            )
            .expect("failed to decode approve response"),
            Ok(_)
        );
    }

    pub fn transfer_tal(
        &self,
        from: impl Into<Account>,
        to: impl Into<Account>,
        amount: u64,
    ) -> Nat {
        let from = from.into();
        let to = to.into();
        Decode!(&assert_reply(self.env.execute_ingress_as(
            PrincipalId::from(from.owner),
            self.taler_ledger_id,
            "icrc1_transfer",
            Encode!(&TransferArg {
                from_subaccount: from.subaccount,
                to,
                fee: None,
                created_at_time: None,
                memo: None,
                amount: Nat::from(amount),
            }).unwrap()
            ).expect("failed to execute token transfer")),
            Result<Nat, TransferError>
        )
        .unwrap()
        .expect("token transfer failed")
    }

    pub fn check_balance_tracking(&self) {
        let ckbtc_balance =
            self.balance_of(self.ckbtc_ledger_id, Principal::from(self.protocol_id));
        let protocol_status = self.get_protocol_status().total_ckbtc_margin;
        let liquidity_status = self
            .get_liquidity_status(Principal::anonymous())
            .total_available_returns;
        assert_eq!(ckbtc_balance, protocol_status + liquidity_status);
    }

    pub fn balance_of(&self, canister_id: CanisterId, from: impl Into<Account>) -> Nat {
        let from = from.into();
        Decode!(
            &assert_reply(
                self.env
                    .execute_ingress_as(
                        PrincipalId::from(from.owner),
                        canister_id,
                        "icrc1_balance_of",
                        Encode!(&from).unwrap()
                    )
                    .expect("failed to execute token transfer")
            ),
            Nat
        )
        .unwrap()
    }

    pub fn borrow_from_vault(
        &self,
        called_by: Principal,
        arg: VaultArg,
    ) -> Result<SuccessWithFee, ProtocolError> {
        Decode!(
            &assert_reply(self.env.execute_ingress_as(
                PrincipalId(called_by),
                self.protocol_id,
                "borrow_from_vault",
                Encode!(&arg).unwrap()
            )
            .expect("failed to borrow from vault")),
            Result<SuccessWithFee, ProtocolError>
        )
        .expect("failed to decode borrow_from_vault response")
    }

    pub fn redeem_ckbtc(
        &self,
        called_by: Principal,
        amount: u64,
    ) -> Result<SuccessWithFee, ProtocolError> {
        Decode!(
            &assert_reply(self.env.execute_ingress_as(
                PrincipalId(called_by),
                self.protocol_id,
                "redeem_ckbtc",
                Encode!(&amount).unwrap()
            )
            .expect("failed to redeem ckbtc")),
            Result<SuccessWithFee, ProtocolError>
        )
        .expect("failed to decode redeem_ckbtc response")
    }

    pub fn provide_liquidity(&self, called_by: Principal, arg: u64) -> Result<u64, ProtocolError> {
        Decode!(
            &assert_reply(self.env.execute_ingress_as(
                PrincipalId(called_by),
                self.protocol_id,
                "provide_liquidity",
                Encode!(&arg).unwrap()
            )
            .expect("failed to provide liquidity")),
            Result<u64, ProtocolError>
        )
        .expect("failed to decode provide_liquidity response")
    }

    pub fn withdraw_liquidity(&self, called_by: Principal, arg: u64) -> Result<u64, ProtocolError> {
        Decode!(
            &assert_reply(self.env.execute_ingress_as(
                PrincipalId(called_by),
                self.protocol_id,
                "withdraw_liquidity",
                Encode!(&arg).unwrap()
            )
            .expect("failed to withdraw liquidity")),
            Result<u64, ProtocolError>
        )
        .expect("failed to decode withdraw_liquidity response")
    }

    pub fn claim_liquidity_returns(&self, called_by: Principal) -> Result<u64, ProtocolError> {
        Decode!(
            &assert_reply(self.env.execute_ingress_as(
                PrincipalId(called_by),
                self.protocol_id,
                "claim_liquidity_returns",
                Encode!().unwrap()
            )
            .expect("failed to claim_liquidity_returns")),
            Result<u64, ProtocolError>
        )
        .expect("failed to decode claim_liquidity_returns response")
    }

    pub fn _get_btc_price(&self) -> GetExchangeRateResult {
        Decode!(
            &assert_reply(
                self.env
                    .execute_ingress(
                        self.xrc_id,
                        "get_exchange_rate",
                        Encode!(&GetExchangeRateRequest {
                            base_asset: Asset {
                                symbol: "BTC".to_string(),
                                class: AssetClass::Cryptocurrency,
                            },
                            quote_asset: Asset {
                                symbol: "USD".to_string(),
                                class: AssetClass::FiatCurrency,
                            },
                            timestamp: None,
                        })
                        .unwrap(),
                    )
                    .expect("failed to get btc price")
            ),
            GetExchangeRateResult
        )
        .expect("could not decode result")
    }

    pub fn get_vaults(&self, called_by: Principal) -> Vec<CandidVault> {
        Decode!(
            &assert_reply(
                self.env
                    .query(self.protocol_id, "get_vaults", Encode!(&called_by).unwrap())
                    .expect("failed to get vaults")
            ),
            Vec<CandidVault>
        )
        .expect("failed to decode get_vaults response")
    }

    pub fn get_vault_history(&self, vault_id: u64) -> Vec<Event> {
        Decode!(
            &assert_reply(
                self.env
                    .query(
                        self.protocol_id,
                        "get_vault_history",
                        Encode!(&vault_id).unwrap()
                    )
                    .expect("failed to get vault events")
            ),
            Vec<Event>
        )
        .expect("failed to decode get_vault_history response")
    }

    pub fn print_events(&self) {
        use crate::GetEventsArg;
        let events = Decode!(
            &assert_reply(
                self.env
                    .query(
                        self.protocol_id,
                        "get_events",
                        Encode!(&GetEventsArg {
                            start: 0,
                            length: 2000,
                        })
                        .unwrap()
                    )
                    .expect("failed to query protocol events")
            ),
            Vec<Event>
        )
        .unwrap();
        println!("{:#?}", events);
    }

    pub fn get_protocol_status(&self) -> ProtocolStatus {
        Decode!(
            &assert_reply(
                self.env
                    .query(self.protocol_id, "get_protocol_status", Encode!().unwrap())
                    .expect("failed to query protocol status")
            ),
            ProtocolStatus
        )
        .expect("failed to decode protocol status")
    }

    pub fn get_fees(&self, redeemed_amount: u64) -> Fees {
        Decode!(
            &assert_reply(
                self.env
                    .query(
                        self.protocol_id,
                        "get_fees",
                        Encode!(&redeemed_amount).unwrap()
                    )
                    .expect("failed to query protocol fees")
            ),
            Fees
        )
        .expect("failed to decode protocol fees")
    }

    pub fn get_liquidity_status(&self, owner: Principal) -> LiquidityStatus {
        Decode!(
            &assert_reply(
                self.env
                    .query(
                        self.protocol_id,
                        "get_liquidity_status",
                        Encode!(&owner).unwrap()
                    )
                    .expect("failed to query liquidity status")
            ),
            LiquidityStatus
        )
        .expect("failed to decode liquidity status")
    }

    pub fn _get_logs(&self) -> Log {
        let request = HttpRequest {
            method: "".to_string(),
            url: "/logs".to_string(),
            headers: vec![],
            body: serde_bytes::ByteBuf::new(),
        };
        let response = Decode!(
            &assert_reply(
                self.env
                    .query(self.protocol_id, "http_request", Encode!(&request).unwrap())
                    .expect("failed to get protocol info")
            ),
            HttpResponse
        )
        .unwrap();
        serde_json::from_slice(&response.body).expect("failed to parse protocol log")
    }

    pub fn _print_protocol_logs(&self) {
        let log = self._get_logs();
        for entry in log.entries {
            println!(
                "{} {}:{} {}",
                entry.timestamp, entry.file, entry.line, entry.message
            );
        }
    }
}

#[test]
fn basic_flow() {
    let elliptic = EllipticSetup::new();

    // Open a new vault

    let open_vault_result = elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S);
    assert_matches!(
        open_vault_result,
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11
        })
    );

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);

    let open_vault_result = elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S);
    assert_matches!(open_vault_result, Ok(_));
    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 2);

    // Borrow max amount from vault

    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount: TAL =
        (ONE_CKBTC * INITIAL_BTC_RATE) / LIQUIDATION_COLLATERAL_RATIO;
    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: maximum_borrowable_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));
    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 1,
            amount: maximum_borrowable_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    let balance_before_closing_vault =
        elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[0]);

    // Close Vault

    let close_vault_result = elliptic.approve_elliptic_and_close_vault(elliptic.principals[0], 0);
    assert_matches!(close_vault_result, Ok(_));

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);

    elliptic.advance_time_and_tick(60);

    let balance_after_closing_vault =
        elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[0]);
    assert_eq!(
        balance_after_closing_vault - balance_before_closing_vault,
        (ONE_CKBTC - CKBTC_TRANSFER_FEE).to_nat()
    );

    elliptic.print_events();

    let vault_events = elliptic.get_vault_history(0);
    assert_eq!(vault_events.len(), 4);

    elliptic.check_balance_tracking();
}

#[test]
fn caller_is_not_vault_owner() {
    let elliptic = EllipticSetup::new();

    // Open a new vault

    let open_vault_result = elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S);
    assert_matches!(
        open_vault_result,
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11
        })
    );

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);

    // Borrow max amount from vault

    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount: TAL =
        (ONE_CKBTC * INITIAL_BTC_RATE) / LIQUIDATION_COLLATERAL_RATIO;

    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: maximum_borrowable_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    assert_matches!(
        elliptic.borrow_from_vault(
            elliptic.principals[1],
            VaultArg {
                vault_id: 0,
                amount: maximum_borrowable_amount.to_u64(),
            },
        ),
        Err(ProtocolError::CallerNotOwner)
    );

    // TODO CHECK ALL ENDPOINTS
}

#[test]
fn liquidate_vault_no_liquidity_pool() {
    let elliptic = EllipticSetup::new();

    // Open a new vault

    for k in 0..5 {
        assert_eq!(
            elliptic
                .approve_ckbtc_and_open_vault(elliptic.principals[0], E8S)
                .expect("open vault failed"),
            OpenVaultSuccess {
                vault_id: k,
                block_index: 11 + 2 * k,
            }
        );
    }
    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 5);

    let current_btc_price = UsdBtc::from(dec!(40_000));
    elliptic.set_btc_price(current_btc_price);
    elliptic.advance_time_and_tick(60);

    // Another user opens a vault at 40k$

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[1], E8S),
        Ok(OpenVaultSuccess { .. })
    );
    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 1);
    let vault_id = vaults[0].vault_id;
    let maximum_borrowable_amount: TAL =
        (ONE_CKBTC * current_btc_price) / LIQUIDATION_COLLATERAL_RATIO;

    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[1],
        VaultArg {
            vault_id,
            amount: maximum_borrowable_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    // Price goes back to 20k$, user 1 gets liquidated

    elliptic.set_btc_price(INITIAL_BTC_RATE);
    elliptic.advance_time_and_tick(60);

    // Check the expected state

    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 0);

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 5);
    let total_ckbtc_margin = vaults.iter().map(|v| v.ckbtc_margin_amount).sum::<u64>();
    let total_borrowed_tal = vaults.iter().map(|v| v.borrowed_tal_amount).sum::<u64>();
    assert_eq!(total_ckbtc_margin, 6 * E8S);
    assert_eq!(total_borrowed_tal, maximum_borrowable_amount);

    elliptic.check_balance_tracking();
}

#[test]
fn liquidate_vault_with_liquidity_pool() {
    let elliptic = EllipticSetup::new();

    let first_vault_ckbtc_margin: CKBTC = CKBTC::from(3 * E8S);
    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(
            elliptic.principals[0],
            first_vault_ckbtc_margin.to_u64()
        ),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );

    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount_first_vault =
        (first_vault_ckbtc_margin * INITIAL_BTC_RATE) / LIQUIDATION_COLLATERAL_RATIO;

    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: maximum_borrowable_amount_first_vault.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    elliptic.tal_approve_elliptic(elliptic.principals[0]);
    assert_matches!(
        elliptic.provide_liquidity(
            elliptic.principals[0],
            (maximum_borrowable_amount_first_vault - TAL_TRANSFER_FEE).to_u64()
        ),
        Ok(_)
    );
    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);

    let first_vault_ckbtc_margin: CKBTC = CKBTC::from(3 * E8S);
    elliptic
        .approve_ckbtc_and_open_vault(elliptic.principals[3], first_vault_ckbtc_margin.to_u64())
        .expect("failed to borrow");
    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount_first_vault =
        (first_vault_ckbtc_margin * INITIAL_BTC_RATE) / LIQUIDATION_COLLATERAL_RATIO;

    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[3],
        VaultArg {
            vault_id: 1,
            amount: maximum_borrowable_amount_first_vault.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    elliptic.tal_approve_elliptic(elliptic.principals[3]);
    assert_matches!(
        elliptic.provide_liquidity(
            elliptic.principals[3],
            (maximum_borrowable_amount_first_vault - TAL_TRANSFER_FEE).to_u64()
        ),
        Ok(_)
    );
    let vaults = elliptic.get_vaults(elliptic.principals[3]);
    assert_eq!(vaults.len(), 1);

    // Another user opens a vault at 40k$

    let current_btc_price = UsdBtc::from(dec!(40_000));
    elliptic.set_btc_price(current_btc_price);
    elliptic.advance_time_and_tick(60);

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[1], E8S),
        Ok(OpenVaultSuccess { .. })
    );
    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 1);
    let vault_id = vaults[0].vault_id;
    let maximum_borrowable_amount = (ONE_CKBTC * current_btc_price) / LIQUIDATION_COLLATERAL_RATIO;

    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[1],
        VaultArg {
            vault_id,
            amount: maximum_borrowable_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    let current_btc_price = UsdBtc::from(dec!(29_726));
    elliptic.set_btc_price(current_btc_price);
    elliptic.advance_time_and_tick(60);

    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 0);
    let protocol_status = elliptic.get_protocol_status();
    assert_eq!(protocol_status.mode, Mode::GeneralAvailability);
    assert!(protocol_status.total_collateral_ratio > 1.5);

    let status = elliptic.get_liquidity_status(elliptic.principals[0]);

    elliptic.check_balance_tracking();
    assert_eq!(status.available_liquidity_reward, E8S / 2);
    assert_eq!(
        status.total_available_returns + protocol_status.total_ckbtc_margin,
        7 * E8S
    );
    assert_eq!(
        status.liquidity_provided,
        maximum_borrowable_amount_first_vault
            - TAL_TRANSFER_FEE
            - maximum_borrowable_amount / Ratio::from(dec!(2))
            - 1.into()
    );
}

#[test]
fn not_borrowing_cannot_liquidate() {
    let elliptic = EllipticSetup::new();

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );

    elliptic.advance_time_and_tick(60);
    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);

    elliptic.set_btc_price(UsdBtc::from(dec!(0)));
    elliptic.advance_time_and_tick(60);

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);

    elliptic.set_btc_price(UsdBtc::from(dec!(100_000_000)));
    elliptic.advance_time_and_tick(60);

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);
}

#[test]
fn provide_and_withdraw_liquidity() {
    let elliptic = EllipticSetup::new();

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );

    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount: TAL =
        (ONE_CKBTC * INITIAL_BTC_RATE) / LIQUIDATION_COLLATERAL_RATIO;
    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: maximum_borrowable_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    elliptic.tal_approve_elliptic(elliptic.principals[0]);
    let balance_before = elliptic.balance_of(elliptic.taler_ledger_id, elliptic.principals[0]);
    assert_matches!(
        elliptic.provide_liquidity(
            elliptic.principals[0],
            (maximum_borrowable_amount - TAL_TRANSFER_FEE).to_u64()
        ),
        Ok(_)
    );
    let status = elliptic.get_liquidity_status(elliptic.principals[0]);
    assert_eq!(
        status.liquidity_provided,
        maximum_borrowable_amount - TAL_TRANSFER_FEE
    );
    let balance = elliptic.balance_of(elliptic.taler_ledger_id, elliptic.principals[0]);
    assert_eq!(balance, 0);

    assert_matches!(
        elliptic.withdraw_liquidity(
            elliptic.principals[0],
            (maximum_borrowable_amount - TAL_TRANSFER_FEE).to_u64()
        ),
        Ok(_)
    );
    let balance = elliptic.balance_of(elliptic.taler_ledger_id, elliptic.principals[0]);
    assert_eq!(balance_before, balance);
}

#[test]
fn protocol_mode() {
    let elliptic = EllipticSetup::new();

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );

    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount: TAL =
        (ONE_CKBTC * INITIAL_BTC_RATE) / LIQUIDATION_COLLATERAL_RATIO;
    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: maximum_borrowable_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    elliptic.set_btc_price(UsdBtc::from(Decimal::ZERO));
    elliptic.advance_time_and_tick(60);

    let protocol_status = elliptic.get_protocol_status();

    assert_eq!(protocol_status.mode, Mode::ReadOnly);
}

#[test]
fn liquidity_returns() {
    let elliptic = EllipticSetup::new();

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], FIVE_E8S),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );
    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[1], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 1,
            block_index: 13,
        })
    );

    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount =
        CKBTC::from(FIVE_E8S) * INITIAL_BTC_RATE / RECOVERY_COLLATERAL_RATIO;

    assert_matches!(
        elliptic.borrow_from_vault(
            elliptic.principals[0],
            VaultArg {
                vault_id: 0,
                amount: maximum_borrowable_amount.to_u64(),
            },
        ),
        Ok(_)
    );
    elliptic.tal_approve_elliptic(elliptic.principals[0]);
    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);
    assert_matches!(
        elliptic.provide_liquidity(
            elliptic.principals[0],
            (maximum_borrowable_amount - TAL_TRANSFER_FEE).to_u64()
        ),
        Ok(_)
    );
    let current_btc_price = UsdBtc::from(dec!(40_000));
    elliptic.set_btc_price(current_btc_price);
    let maximum_borrowable_amount = ONE_CKBTC * current_btc_price / LIQUIDATION_COLLATERAL_RATIO;
    elliptic.advance_time_and_tick(60);

    assert_matches!(
        elliptic.borrow_from_vault(
            elliptic.principals[1],
            VaultArg {
                vault_id: 1,
                amount: maximum_borrowable_amount.to_u64(),
            },
        ),
        Ok(_)
    );

    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 1);
    elliptic.set_btc_price(INITIAL_BTC_RATE);
    elliptic.advance_time_and_tick(60);

    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 0);

    assert_matches!(
        elliptic.claim_liquidity_returns(elliptic.principals[0]),
        Ok(_)
    );
    assert_matches!(
        elliptic.env.execute_ingress_as(
            PrincipalId(elliptic.principals[0]),
            elliptic.protocol_id,
            "claim_liquidity_returns",
            Encode!().unwrap()
        ),
        Err(_)
    );
}

#[test]
fn automatic_mode_change() {
    let elliptic = EllipticSetup::new();
    elliptic.advance_time_and_tick(60);

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );

    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount: TAL =
        (ONE_CKBTC * INITIAL_BTC_RATE) / LIQUIDATION_COLLATERAL_RATIO;
    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: maximum_borrowable_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    elliptic.set_btc_price(UsdBtc::from(Decimal::ZERO));
    elliptic.advance_time_and_tick(60);

    let protocol_status = elliptic.get_protocol_status();
    assert_eq!(protocol_status.mode, Mode::ReadOnly);
}

#[test]
fn reject_request_if_btc_price_too_old() {
    use crate::ProtocolError::TemporarilyUnavailable;

    let elliptic = EllipticSetup::new();
    elliptic.advance_time_and_tick(60);

    let protocol_status = elliptic.get_protocol_status();
    assert_ne!(protocol_status.last_btc_timestamp, 0);

    assert_matches!(elliptic.env.stop_canister(elliptic.xrc_id), Ok(_));

    for _ in 0..20 {
        elliptic.advance_time_and_tick(60);
    }

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
        Err(TemporarilyUnavailable(_))
    );

    let protocol_status = elliptic.get_protocol_status();
    assert_eq!(protocol_status.mode, Mode::GeneralAvailability);
}

#[test]
fn vault_id_persist_accross_upgrade() {
    let elliptic = EllipticSetup::new();
    elliptic.advance_time_and_tick(60);

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );

    for _ in 1..6 {
        assert_matches!(
            elliptic.open_vault(elliptic.principals[0], E8S),
            Ok(OpenVaultSuccess { .. })
        );
    }

    assert_matches!(
        elliptic.env.upgrade_canister(
            elliptic.protocol_id,
            protocol_wasm(),
            Encode!(&ProtocolArg::Upgrade(UpgradeArg { mode: None })).unwrap(),
        ),
        Ok(_)
    );
    elliptic.advance_time_and_tick(60);

    assert_matches!(
        elliptic.open_vault(elliptic.principals[0], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 6,
            block_index: 17,
        })
    );
}

#[test]
fn borrow_too_much() {
    let elliptic = EllipticSetup::new();

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );

    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount = ONE_CKBTC * INITIAL_BTC_RATE / LIQUIDATION_COLLATERAL_RATIO;

    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: maximum_borrowable_amount.to_u64() + 1,
        },
    );
    assert_matches!(
        borrow_from_vault_result,
        Err(ProtocolError::GenericError(..))
    );
}

#[test]
fn cannot_forge_rewards() {
    let elliptic = EllipticSetup::new();

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], FIVE_E8S),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );
    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[1], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 1,
            block_index: 13,
        })
    );

    elliptic.advance_time_and_tick(60);
    let maximum_borrowable_amount =
        CKBTC::from(FIVE_E8S) * INITIAL_BTC_RATE / RECOVERY_COLLATERAL_RATIO;
    assert_matches!(
        elliptic.borrow_from_vault(
            elliptic.principals[0],
            VaultArg {
                vault_id: 0,
                amount: maximum_borrowable_amount.to_u64(),
            },
        ),
        Ok(_)
    );
    elliptic.tal_approve_elliptic(elliptic.principals[0]);
    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);
    assert_matches!(
        elliptic.provide_liquidity(
            elliptic.principals[0],
            (maximum_borrowable_amount - TAL_TRANSFER_FEE).to_u64()
        ),
        Ok(_)
    );
    let current_btc_price = UsdBtc::from(dec!(40_000));
    elliptic.set_btc_price(current_btc_price);
    let maximum_borrowable_amount = ONE_CKBTC * current_btc_price / LIQUIDATION_COLLATERAL_RATIO;

    elliptic.advance_time_and_tick(60);

    assert_matches!(
        elliptic.borrow_from_vault(
            elliptic.principals[1],
            VaultArg {
                vault_id: 1,
                amount: maximum_borrowable_amount.to_u64(),
            },
        ),
        Ok(_)
    );

    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 1);
    elliptic.set_btc_price(INITIAL_BTC_RATE);
    elliptic.advance_time_and_tick(60);

    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 0);

    assert_matches!(elliptic.env.stop_canister(elliptic.ckbtc_ledger_id), Ok(_));

    assert!(elliptic
        .claim_liquidity_returns(elliptic.principals[0])
        .is_err());
}

#[test]
fn liquidation_with_lp_in_recovery_mode() {
    let elliptic = EllipticSetup::new();

    let first_vault_ckbtc_margin_amount = CKBTC::from(2 * E8S);
    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(
            elliptic.principals[0],
            first_vault_ckbtc_margin_amount.to_u64()
        ),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );

    elliptic.advance_time_and_tick(60);
    let first_vault_borrow_amount =
        first_vault_ckbtc_margin_amount * INITIAL_BTC_RATE / RECOVERY_COLLATERAL_RATIO;
    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: first_vault_borrow_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    elliptic.tal_approve_elliptic(elliptic.principals[0]);
    assert_matches!(
        elliptic.provide_liquidity(
            elliptic.principals[0],
            (first_vault_borrow_amount - TAL_TRANSFER_FEE).to_u64()
        ),
        Ok(_)
    );
    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);

    let current_btc_price = UsdBtc::from(dec!(20_000));

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[1], E8S),
        Ok(OpenVaultSuccess { .. })
    );
    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 1);
    let vault_id = vaults[0].vault_id;
    let maximum_borrowable_amount = ONE_CKBTC * current_btc_price / LIQUIDATION_COLLATERAL_RATIO;
    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[1],
        VaultArg {
            vault_id,
            amount: maximum_borrowable_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    elliptic.advance_time_and_tick(60);

    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].ckbtc_margin_amount, 1);
    assert_eq!(vaults[0].borrowed_tal_amount, 0);

    let status = elliptic.get_protocol_status();

    assert_eq!(status.total_tal_borrowed, first_vault_borrow_amount);

    let status = elliptic.get_liquidity_status(elliptic.principals[0]);
    assert_eq!(status.available_liquidity_reward, 99999999);
    assert_eq!(
        status.liquidity_provided,
        first_vault_borrow_amount - maximum_borrowable_amount - TAL_TRANSFER_FEE
    );
}

#[test]
fn always_get_expected_mode() {
    let elliptic = EllipticSetup::new();

    let status = elliptic.get_protocol_status();
    assert_eq!(status.mode, Mode::GeneralAvailability);

    let first_vault_ckbtc_margin_amount = CKBTC::from(2 * E8S);
    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(
            elliptic.principals[0],
            first_vault_ckbtc_margin_amount.to_u64()
        ),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11,
        })
    );

    elliptic.advance_time_and_tick(60);
    let first_vault_borrow_amount =
        first_vault_ckbtc_margin_amount * INITIAL_BTC_RATE / RECOVERY_COLLATERAL_RATIO;

    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: first_vault_borrow_amount.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));

    elliptic.advance_time_and_tick(60);

    let status = elliptic.get_protocol_status();
    assert_eq!(status.mode, Mode::GeneralAvailability);

    let current_btc_price = UsdBtc::from(dec!(15_000));
    elliptic.set_btc_price(current_btc_price);
    elliptic.advance_time_and_tick(60);

    // The protocol will switch to recovery.
    let status = elliptic.get_protocol_status();
    assert_eq!(status.mode, Mode::Recovery);

    // We open a new vault with some margin,
    // the first vault can now be liquidated on this vault.

    let first_vault_ckbtc_margin_amount = 2 * E8S;
    assert_matches!(
        elliptic
            .approve_ckbtc_and_open_vault(elliptic.principals[0], first_vault_ckbtc_margin_amount),
        Ok(OpenVaultSuccess {
            vault_id: 1,
            block_index: 13,
        })
    );

    elliptic.advance_time_and_tick(60);

    let status = elliptic.get_protocol_status();
    assert_eq!(status.mode, Mode::GeneralAvailability);
}

#[test]
fn check_automatic_fee_update() {
    let elliptic = EllipticSetup::new();

    assert_matches!(
        elliptic.env.upgrade_canister(
            elliptic.ckbtc_ledger_id,
            icrc1_ledger_wasm(),
            Encode!(&LedgerArgument::Upgrade(Some(LedgerUpgradeArgs {
                metadata: None,
                token_name: None,
                token_symbol: None,
                transfer_fee: Some(Nat::from(111)),
                change_fee_collector: None,
                max_memo_length: None,
                feature_flags: None,
                maximum_number_of_accounts: None,
                accounts_overflow_trim_quantity: None,
            })))
            .unwrap(),
        ),
        Ok(_)
    );

    // First request should fail as the fee set in the protocol canister is wrong.

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
        Err(ProtocolError::TransferFromError(..))
    );

    // This second request should be ok as the fee should be updated.

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 12,
        })
    );

    let maximum_borrowable_amount_first_vault =
        ONE_CKBTC * INITIAL_BTC_RATE / RECOVERY_COLLATERAL_RATIO;
    let borrow_from_vault_result = elliptic.borrow_from_vault(
        elliptic.principals[0],
        VaultArg {
            vault_id: 0,
            amount: maximum_borrowable_amount_first_vault.to_u64(),
        },
    );
    assert_matches!(borrow_from_vault_result, Ok(_));
}

#[test]
fn should_redeem_ckbtc() {
    let elliptic = EllipticSetup::new();
    let maximum_borrowable_amount_first_vault =
        ONE_CKBTC * INITIAL_BTC_RATE / RECOVERY_COLLATERAL_RATIO;

    for k in 0..5 {
        assert_matches!(
            elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
            Ok(OpenVaultSuccess { .. })
        );
        assert_matches!(
            elliptic.borrow_from_vault(
                elliptic.principals[0],
                VaultArg {
                    vault_id: k,
                    amount: maximum_borrowable_amount_first_vault.to_u64(),
                },
            ),
            Ok(_)
        );
    }

    elliptic.transfer_tal(
        elliptic.principals[0],
        elliptic.principals[1],
        maximum_borrowable_amount_first_vault.to_u64(),
    );

    elliptic.tal_approve_elliptic(elliptic.principals[1]);

    let ckbtc_balance_pre_redemption =
        elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[1]);

    let redeem_amount = (maximum_borrowable_amount_first_vault - TAL_TRANSFER_FEE).to_u64();
    let fees = elliptic.get_fees(redeem_amount);
    let redemption_result = elliptic
        .redeem_ckbtc(elliptic.principals[1], redeem_amount)
        .expect("failed to redeem");
    let expected_fee = TAL::from(redeem_amount)
        * Ratio::from(Decimal::from_f64_retain(fees.redemption_fee).unwrap());
    assert_eq!(redemption_result.fee_amount_paid, expected_fee);

    elliptic.advance_time_and_tick(1);

    let expected_ckbtc_amount =
        (maximum_borrowable_amount_first_vault - expected_fee - TAL_TRANSFER_FEE)
            / INITIAL_BTC_RATE;

    let ckbtc_balance_post_redemption =
        elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[1]);

    assert!(ckbtc_balance_post_redemption > ckbtc_balance_pre_redemption);
    assert_eq!(
        (expected_ckbtc_amount - CKBTC_TRANSFER_FEE).to_nat(),
        ckbtc_balance_post_redemption - ckbtc_balance_pre_redemption
    );

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 5);
    assert_eq!(
        vaults[0].borrowed_tal_amount,
        TAL_TRANSFER_FEE + expected_fee
    );
    assert_eq!(
        vaults[0].ckbtc_margin_amount,
        E8S - expected_ckbtc_amount.to_u64()
    );
}

#[test]
fn should_redeem_on_several_vaults() {
    let elliptic = EllipticSetup::new();

    let maximum_borrowable_amount_first_vault =
        ONE_CKBTC * INITIAL_BTC_RATE / RECOVERY_COLLATERAL_RATIO;

    for k in 0..5 {
        assert_matches!(
            elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
            Ok(OpenVaultSuccess { .. })
        );
        let borrow_result = elliptic
            .borrow_from_vault(
                elliptic.principals[0],
                VaultArg {
                    vault_id: k,
                    amount: maximum_borrowable_amount_first_vault.to_u64(),
                },
            )
            .expect("failed to borrow");
        assert_eq!(borrow_result.fee_amount_paid, 0);
    }

    elliptic.transfer_tal(
        elliptic.principals[0],
        elliptic.principals[1],
        2 * maximum_borrowable_amount_first_vault.to_u64(),
    );

    elliptic.tal_approve_elliptic(elliptic.principals[1]);

    let ckbtc_balance_pre_redemption =
        elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[1]);

    let redeem_amount =
        2 * maximum_borrowable_amount_first_vault.to_u64() - TAL_TRANSFER_FEE.to_u64();

    let fees = elliptic.get_fees(redeem_amount);
    let expected_fee = TAL::from(redeem_amount)
        * Ratio::from(Decimal::from_f64_retain(fees.redemption_fee).unwrap());

    let redemption_result = elliptic
        .redeem_ckbtc(elliptic.principals[1], redeem_amount)
        .expect("failed to redeem");
    assert_eq!(redemption_result.fee_amount_paid, expected_fee);

    assert_eq!(
        Nat::from(0),
        elliptic.balance_of(elliptic.taler_ledger_id, elliptic.principals[1])
    );

    elliptic.advance_time_and_tick(1);

    let expected_ckbtc_amount = (TAL::from(redeem_amount) - expected_fee) / INITIAL_BTC_RATE;

    let ckbtc_balance_post_redemption =
        elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[1]);

    assert!(ckbtc_balance_post_redemption > ckbtc_balance_pre_redemption);
    assert_eq!(
        (expected_ckbtc_amount - CKBTC_TRANSFER_FEE).to_nat(),
        ckbtc_balance_post_redemption - ckbtc_balance_pre_redemption
    );

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 5);
    assert_eq!(vaults[0].borrowed_tal_amount, 0);
    let expeced_amount = maximum_borrowable_amount_first_vault.to_u64()
        - (redeem_amount - maximum_borrowable_amount_first_vault.to_u64() - expected_fee.to_u64());
    assert_eq!(vaults[1].borrowed_tal_amount, expeced_amount);
}

#[test]
fn fees_are_as_expected() {
    let mut elliptic = EllipticSetup::new();
    elliptic.with_fee(500_000);

    let initial_fees = elliptic.get_fees(0);
    assert_eq!(initial_fees.borrowing_fee, 0.005);
    assert_eq!(initial_fees.redemption_fee, 0.0);

    let current_btc_price = UsdBtc::from(dec!(10_000_000));
    elliptic.set_btc_price(current_btc_price);
    elliptic.advance_time_and_tick(60);

    let ckbtc_margin_amount: u64 = 5 * E8S;

    let open_vault_result =
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], ckbtc_margin_amount);
    assert_matches!(
        open_vault_result,
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11
        })
    );

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].borrowed_tal_amount, 0);
    assert_eq!(vaults[0].ckbtc_margin_amount, ckbtc_margin_amount);

    // Borrow max amount from vault

    elliptic.advance_time_and_tick(60);
    let borrow_amount: TAL = TAL::from(20_000_000 * E8S);
    let borrow_from_vault_result = elliptic
        .borrow_from_vault(
            elliptic.principals[0],
            VaultArg {
                vault_id: 0,
                amount: borrow_amount.to_u64(),
            },
        )
        .expect("failed to borrow");
    assert_eq!(borrow_from_vault_result.fee_amount_paid, 100_000 * E8S);

    let liquidity_dev = elliptic.get_liquidity_status(Principal::anonymous());
    assert_eq!(liquidity_dev.liquidity_provided, 100_000 * E8S);

    elliptic.tal_approve_elliptic(elliptic.principals[0]);

    assert_matches!(
        elliptic.redeem_ckbtc(elliptic.principals[0], 723_379 * E8S),
        Ok(_)
    );

    elliptic.advance_time_and_tick(2 * 60 * 60);

    let fees = elliptic.get_fees(150_000 * E8S);
    assert_eq!(fees.borrowing_fee, 0.005);
    assert_eq!(fees.redemption_fee, 0.019867526870783477);
}

#[test]
fn vault_round_trip() {
    let elliptic = EllipticSetup::new();

    let initial_balance = elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[0]);

    let open_vault_result = elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S);
    assert_matches!(
        open_vault_result,
        Ok(OpenVaultSuccess {
            vault_id: 0,
            block_index: 11
        })
    );

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);

    assert_matches!(elliptic.close_vault(elliptic.principals[0], 0), Ok(_));
    elliptic.advance_time_and_tick(1);

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 0);

    let end_balance = elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[0]);
    let fees = CKBTC_TRANSFER_FEE * Ratio::new(dec!(3.0));
    assert_eq!(initial_balance - end_balance, fees.to_nat());
}

#[test]
fn double_redemption() {
    let elliptic = EllipticSetup::new();
    let maximum_borrowable_amount_first_vault =
        ONE_CKBTC * INITIAL_BTC_RATE / RECOVERY_COLLATERAL_RATIO;

    for k in 0..5 {
        assert_matches!(
            elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], E8S),
            Ok(OpenVaultSuccess { .. })
        );
        assert_matches!(
            elliptic.borrow_from_vault(
                elliptic.principals[0],
                VaultArg {
                    vault_id: k,
                    amount: maximum_borrowable_amount_first_vault.to_u64(),
                },
            ),
            Ok(_)
        );
    }

    elliptic.transfer_tal(
        elliptic.principals[0],
        elliptic.principals[1],
        2 * maximum_borrowable_amount_first_vault.to_u64(),
    );

    elliptic.tal_approve_elliptic(elliptic.principals[1]);

    let ckbtc_balance_pre_redemption =
        elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[1]);

    let redeem_amount = (maximum_borrowable_amount_first_vault - TAL_TRANSFER_FEE).to_u64();
    let fees = elliptic.get_fees(redeem_amount);
    let redemption_result = elliptic
        .redeem_ckbtc(elliptic.principals[1], redeem_amount)
        .expect("failed to redeem");
    let expected_fee = TAL::from(redeem_amount)
        * Ratio::from(Decimal::from_f64_retain(fees.redemption_fee).unwrap());
    assert_eq!(redemption_result.fee_amount_paid, expected_fee);

    let redeem_amount = (maximum_borrowable_amount_first_vault - TAL_TRANSFER_FEE).to_u64();
    let fees = elliptic.get_fees(redeem_amount);
    let redemption_result = elliptic
        .redeem_ckbtc(elliptic.principals[1], redeem_amount)
        .expect("failed to redeem");
    let expected_fee = TAL::from(redeem_amount)
        * Ratio::from(Decimal::from_f64_retain(fees.redemption_fee).unwrap());
    assert_eq!(redemption_result.fee_amount_paid, expected_fee);

    elliptic.advance_time_and_tick(1);

    let expected_ckbtc_amount =
        (maximum_borrowable_amount_first_vault - expected_fee - TAL_TRANSFER_FEE)
            / INITIAL_BTC_RATE;

    let ckbtc_balance_post_redemption =
        elliptic.balance_of(elliptic.ckbtc_ledger_id, elliptic.principals[1]);

    assert!(ckbtc_balance_post_redemption > ckbtc_balance_pre_redemption);
    assert_eq!(
        (expected_ckbtc_amount * Ratio::from(dec!(2.0))
            - CKBTC_TRANSFER_FEE * Ratio::from(dec!(2.0)))
        .to_nat(),
        ckbtc_balance_post_redemption - ckbtc_balance_pre_redemption
    );

    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 5);
    assert_eq!(
        vaults[0].borrowed_tal_amount,
        TAL_TRANSFER_FEE + expected_fee
    );
    assert_eq!(
        vaults[0].ckbtc_margin_amount,
        E8S - expected_ckbtc_amount.to_u64()
    );

    assert_eq!(
        vaults[1].borrowed_tal_amount,
        TAL_TRANSFER_FEE + expected_fee
    );
    assert_eq!(
        vaults[1].ckbtc_margin_amount,
        E8S - expected_ckbtc_amount.to_u64()
    );

    assert_eq!(
        vaults[2].borrowed_tal_amount,
        maximum_borrowable_amount_first_vault.to_u64()
    );
    assert_eq!(vaults[2].ckbtc_margin_amount, E8S);
}

#[test]
fn should_liquidate_vault_below_recovery_cr() {
    let elliptic = EllipticSetup::new();

    let ckbtc_margin_first_vault = 5 * E8S;

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[0], ckbtc_margin_first_vault),
        Ok(OpenVaultSuccess { .. })
    );

    let borrow_amount = 70_000 * E8S;
    assert_matches!(
        elliptic.borrow_from_vault(
            elliptic.principals[0],
            VaultArg {
                vault_id: 0,
                amount: borrow_amount,
            },
        ),
        Ok(_)
    );

    elliptic.tal_approve_elliptic(elliptic.principals[0]);
    assert_matches!(
        elliptic.provide_liquidity(
            elliptic.principals[0],
            borrow_amount - TAL_TRANSFER_FEE.to_u64()
        ),
        Ok(_)
    );
    let vaults = elliptic.get_vaults(elliptic.principals[0]);
    assert_eq!(vaults.len(), 1);

    assert_matches!(
        elliptic.approve_ckbtc_and_open_vault(elliptic.principals[1], E8S),
        Ok(OpenVaultSuccess { .. })
    );
    let borrow_amount = 18181 * E8S; // Vault should have a cr just below 150%
    assert_matches!(
        elliptic.borrow_from_vault(
            elliptic.principals[1],
            VaultArg {
                vault_id: 1,
                amount: borrow_amount,
            },
        ),
        Ok(_)
    );

    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 1);

    elliptic.advance_time_and_tick(60);
    let vaults = elliptic.get_vaults(elliptic.principals[1]);
    assert_eq!(vaults.len(), 1);
    assert_eq!(vaults[0].borrowed_tal_amount, 0);
}
