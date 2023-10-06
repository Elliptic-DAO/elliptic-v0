export const idlFactory = ({ IDL }) => {
    const Mode = IDL.Variant({
        'ReadOnly': IDL.Null,
        'GeneralAvailability': IDL.Null,
        'Recovery': IDL.Null,
    });
    const UpgradeArg = IDL.Record({ 'mode': IDL.Opt(Mode) });
    const InitArg = IDL.Record({
        'ckbtc_ledger_principal': IDL.Principal,
        'xrc_principal': IDL.Principal,
        'fee_e8s': IDL.Nat64,
        'taler_ledger_principal': IDL.Principal,
        'developer_principal': IDL.Principal,
    });
    const ProtocolArg = IDL.Variant({ 'Upgrade': UpgradeArg, 'Init': InitArg });
    const VaultArg = IDL.Record({ 'vault_id': IDL.Nat64, 'amount': IDL.Nat64 });
    const TransferError = IDL.Variant({
        'GenericError': IDL.Record({
            'message': IDL.Text,
            'error_code': IDL.Nat,
        }),
        'TemporarilyUnavailable': IDL.Null,
        'BadBurn': IDL.Record({ 'min_burn_amount': IDL.Nat }),
        'Duplicate': IDL.Record({ 'duplicate_of': IDL.Nat }),
        'BadFee': IDL.Record({ 'expected_fee': IDL.Nat }),
        'CreatedInFuture': IDL.Record({ 'ledger_time': IDL.Nat64 }),
        'TooOld': IDL.Null,
        'InsufficientFunds': IDL.Record({ 'balance': IDL.Nat }),
    });
    const TransferFromError = IDL.Variant({
        'GenericError': IDL.Record({
            'message': IDL.Text,
            'error_code': IDL.Nat,
        }),
        'TemporarilyUnavailable': IDL.Null,
        'InsufficientAllowance': IDL.Record({ 'allowance': IDL.Nat }),
        'BadBurn': IDL.Record({ 'min_burn_amount': IDL.Nat }),
        'Duplicate': IDL.Record({ 'duplicate_of': IDL.Nat }),
        'BadFee': IDL.Record({ 'expected_fee': IDL.Nat }),
        'CreatedInFuture': IDL.Record({ 'ledger_time': IDL.Nat64 }),
        'TooOld': IDL.Null,
        'InsufficientFunds': IDL.Record({ 'balance': IDL.Nat }),
    });
    const ProtocolError = IDL.Variant({
        'GenericError': IDL.Text,
        'TemporarilyUnavailable': IDL.Text,
        'TransferError': TransferError,
        'AlreadyProcessing': IDL.Null,
        'AnonymousCallerNotAllowed': IDL.Null,
        'AmountTooLow': IDL.Record({ 'minimum_amount': IDL.Nat64 }),
        'TransferFromError': IDL.Tuple(TransferFromError, IDL.Nat64),
        'CallerNotOwner': IDL.Null,
    });
    const SuccessWithFee = IDL.Record({
        'block_index': IDL.Nat64,
        'fee_amount_paid': IDL.Nat64,
    });
    const GetEventsArg = IDL.Record({
        'start': IDL.Nat64,
        'length': IDL.Nat64,
    });
    const Vault = IDL.Record({
        'owner': IDL.Principal,
        'vault_id': IDL.Nat64,
        'borrowed_tal_amount': IDL.Nat64,
        'ckbtc_margin_amount': IDL.Nat64,
    });
    const Event = IDL.Variant({
        'claim_liquidity_returns': IDL.Record({
            'block_index': IDL.Nat64,
            'caller': IDL.Principal,
            'amount': IDL.Nat64,
        }),
        'repay_to_vault': IDL.Record({
            'block_index': IDL.Nat64,
            'vault_id': IDL.Nat64,
            'repayed_amount': IDL.Nat64,
        }),
        'provide_liquidity': IDL.Record({
            'block_index': IDL.Nat64,
            'caller': IDL.Principal,
            'amount': IDL.Nat64,
        }),
        'init': InitArg,
        'open_vault': IDL.Record({ 'block_index': IDL.Nat64, 'vault': Vault }),
        'redemption_on_vaults': IDL.Record({
            'owner': IDL.Principal,
            'tal_block_index': IDL.Nat64,
            'tal_amount': IDL.Nat64,
            'fee_amount': IDL.Nat64,
            'current_btc_rate': IDL.Vec(IDL.Nat8),
        }),
        'margin_transfer': IDL.Record({
            'block_index': IDL.Nat64,
            'vault_id': IDL.Nat64,
        }),
        'upgrade': UpgradeArg,
        'borrow_from_vault': IDL.Record({
            'block_index': IDL.Nat64,
            'vault_id': IDL.Nat64,
            'fee_amount': IDL.Nat64,
            'borrowed_amount': IDL.Nat64,
        }),
        'redistribute_vault': IDL.Record({ 'vault_id': IDL.Nat64 }),
        'withdraw_liquidity': IDL.Record({
            'block_index': IDL.Nat64,
            'caller': IDL.Principal,
            'amount': IDL.Nat64,
        }),
        'close_vault': IDL.Record({
            'block_index': IDL.Opt(IDL.Nat64),
            'vault_id': IDL.Nat64,
        }),
        'add_margin_to_vault': IDL.Record({
            'block_index': IDL.Nat64,
            'vault_id': IDL.Nat64,
            'margin_added': IDL.Nat64,
        }),
        'redemption_transfered': IDL.Record({
            'tal_block_index': IDL.Nat64,
            'ckbtc_block_index': IDL.Nat64,
        }),
        'liquidate_vault': IDL.Record({
            'mode': Mode,
            'btc_rate': IDL.Vec(IDL.Nat8),
            'vault_id': IDL.Nat64,
        }),
    });
    const Fees = IDL.Record({
        'redemption_fee': IDL.Float64,
        'borrowing_fee': IDL.Float64,
    });
    const LiquidityStatus = IDL.Record({
        'liquidity_provided': IDL.Nat64,
        'total_liquidity_provided': IDL.Nat64,
        'liquidity_pool_share': IDL.Float64,
        'available_liquidity_reward': IDL.Nat64,
        'total_available_returns': IDL.Nat64,
    });
    const ProtocolStatus = IDL.Record({
        'mode': Mode,
        'total_tal_borrowed': IDL.Nat64,
        'total_collateral_ratio': IDL.Float64,
        'total_ckbtc_margin': IDL.Nat64,
        'last_btc_timestamp': IDL.Nat64,
        'last_btc_rate': IDL.Float64,
    });
    const OpenVaultSuccess = IDL.Record({
        'block_index': IDL.Nat64,
        'vault_id': IDL.Nat64,
    });
    return IDL.Service({
        'add_margin_to_vault': IDL.Func(
            [VaultArg],
            [IDL.Variant({ 'Ok': IDL.Nat64, 'Err': ProtocolError })],
            [],
        ),
        'borrow_from_vault': IDL.Func(
            [VaultArg],
            [IDL.Variant({ 'Ok': SuccessWithFee, 'Err': ProtocolError })],
            [],
        ),
        'claim_liquidity_returns': IDL.Func(
            [],
            [IDL.Variant({ 'Ok': IDL.Nat64, 'Err': ProtocolError })],
            [],
        ),
        'close_vault': IDL.Func(
            [IDL.Nat64],
            [IDL.Variant({ 'Ok': IDL.Opt(IDL.Nat64), 'Err': ProtocolError })],
            [],
        ),
        'get_events': IDL.Func([GetEventsArg], [IDL.Vec(Event)], ['query']),
        'get_fees': IDL.Func([IDL.Nat64], [Fees], ['query']),
        'get_liquidity_status': IDL.Func(
            [IDL.Principal],
            [LiquidityStatus],
            ['query'],
        ),
        'get_protocol_status': IDL.Func([], [ProtocolStatus], ['query']),
        'get_vault_history': IDL.Func([IDL.Nat64], [IDL.Vec(Event)], ['query']),
        'get_vaults': IDL.Func(
            [IDL.Opt(IDL.Principal)],
            [IDL.Vec(Vault)],
            ['query'],
        ),
        'open_vault': IDL.Func(
            [IDL.Nat64],
            [IDL.Variant({ 'Ok': OpenVaultSuccess, 'Err': ProtocolError })],
            [],
        ),
        'provide_liquidity': IDL.Func(
            [IDL.Nat64],
            [IDL.Variant({ 'Ok': IDL.Nat64, 'Err': ProtocolError })],
            [],
        ),
        'redeem_ckbtc': IDL.Func(
            [IDL.Nat64],
            [IDL.Variant({ 'Ok': SuccessWithFee, 'Err': ProtocolError })],
            [],
        ),
        'repay_to_vault': IDL.Func(
            [VaultArg],
            [IDL.Variant({ 'Ok': IDL.Nat64, 'Err': ProtocolError })],
            [],
        ),
        'withdraw_liquidity': IDL.Func(
            [IDL.Nat64],
            [IDL.Variant({ 'Ok': IDL.Nat64, 'Err': ProtocolError })],
            [],
        ),
    });
};
export const init = ({ IDL }) => {
    const Mode = IDL.Variant({
        'ReadOnly': IDL.Null,
        'GeneralAvailability': IDL.Null,
        'Recovery': IDL.Null,
    });
    const UpgradeArg = IDL.Record({ 'mode': IDL.Opt(Mode) });
    const InitArg = IDL.Record({
        'ckbtc_ledger_principal': IDL.Principal,
        'xrc_principal': IDL.Principal,
        'fee_e8s': IDL.Nat64,
        'taler_ledger_principal': IDL.Principal,
        'developer_principal': IDL.Principal,
    });
    const ProtocolArg = IDL.Variant({ 'Upgrade': UpgradeArg, 'Init': InitArg });
    return [ProtocolArg];
};
