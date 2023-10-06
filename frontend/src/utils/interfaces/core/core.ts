import type { Principal } from '@dfinity/principal';
import type { ActorMethod } from '@dfinity/agent';

export type Event = {
    'claim_liquidity_returns': {
        'block_index': bigint,
        'caller': Principal,
        'amount': bigint,
    }
} |
{
    'repay_to_vault': {
        'block_index': bigint,
        'vault_id': bigint,
        'repayed_amount': bigint,
    }
} |
{
    'provide_liquidity': {
        'block_index': bigint,
        'caller': Principal,
        'amount': bigint,
    }
} |
{ 'init': InitArg } |
{ 'open_vault': { 'block_index': bigint, 'vault': Vault } } |
{
    'redemption_on_vaults': {
        'owner': Principal,
        'tal_block_index': bigint,
        'tal_amount': bigint,
        'fee_amount': bigint,
        'current_btc_rate': Uint8Array | number[],
    }
} |
{ 'margin_transfer': { 'block_index': bigint, 'vault_id': bigint } } |
{ 'upgrade': UpgradeArg } |
{
    'borrow_from_vault': {
        'block_index': bigint,
        'vault_id': bigint,
        'fee_amount': bigint,
        'borrowed_amount': bigint,
    }
} |
{ 'redistribute_vault': { 'vault_id': bigint } } |
{
    'withdraw_liquidity': {
        'block_index': bigint,
        'caller': Principal,
        'amount': bigint,
    }
} |
{ 'close_vault': { 'block_index': [] | [bigint], 'vault_id': bigint } } |
{
    'add_margin_to_vault': {
        'block_index': bigint,
        'vault_id': bigint,
        'margin_added': bigint,
    }
} |
{
    'redemption_transfered': {
        'tal_block_index': bigint,
        'ckbtc_block_index': bigint,
    }
} |
{
    'liquidate_vault': {
        'mode': Mode,
        'btc_rate': Uint8Array | number[],
        'vault_id': bigint,
    }
};
export interface Fees { 'redemption_fee': number, 'borrowing_fee': number }
export interface GetEventsArg { 'start': bigint, 'length': bigint }
export interface InitArg {
    'ckbtc_ledger_principal': Principal,
    'xrc_principal': Principal,
    'fee_e8s': bigint,
    'taler_ledger_principal': Principal,
    'developer_principal': Principal,
}
export interface LiquidityStatus {
    'liquidity_provided': bigint,
    'total_liquidity_provided': bigint,
    'liquidity_pool_share': number,
    'available_liquidity_reward': bigint,
    'total_available_returns': bigint,
}
export type Mode = { 'ReadOnly': null } |
{ 'GeneralAvailability': null } |
{ 'Recovery': null };
export interface OpenVaultSuccess {
    'block_index': bigint,
    'vault_id': bigint,
}
export type ProtocolArg = { 'Upgrade': UpgradeArg } |
{ 'Init': InitArg };
export type ProtocolError = { 'GenericError': string } |
{ 'TemporarilyUnavailable': string } |
{ 'TransferError': TransferError } |
{ 'AlreadyProcessing': null } |
{ 'AnonymousCallerNotAllowed': null } |
{ 'AmountTooLow': { 'minimum_amount': bigint } } |
{ 'TransferFromError': [TransferFromError, bigint] } |
{ 'CallerNotOwner': null };
export interface ProtocolStatus {
    'mode': Mode,
    'total_tal_borrowed': bigint,
    'total_collateral_ratio': number,
    'total_ckbtc_margin': bigint,
    'last_btc_timestamp': bigint,
    'last_btc_rate': number,
}
export interface SuccessWithFee {
    'block_index': bigint,
    'fee_amount_paid': bigint,
}
export type TransferError = {
    'GenericError': { 'message': string, 'error_code': bigint }
} |
{ 'TemporarilyUnavailable': null } |
{ 'BadBurn': { 'min_burn_amount': bigint } } |
{ 'Duplicate': { 'duplicate_of': bigint } } |
{ 'BadFee': { 'expected_fee': bigint } } |
{ 'CreatedInFuture': { 'ledger_time': bigint } } |
{ 'TooOld': null } |
{ 'InsufficientFunds': { 'balance': bigint } };
export type TransferFromError = {
    'GenericError': { 'message': string, 'error_code': bigint }
} |
{ 'TemporarilyUnavailable': null } |
{ 'InsufficientAllowance': { 'allowance': bigint } } |
{ 'BadBurn': { 'min_burn_amount': bigint } } |
{ 'Duplicate': { 'duplicate_of': bigint } } |
{ 'BadFee': { 'expected_fee': bigint } } |
{ 'CreatedInFuture': { 'ledger_time': bigint } } |
{ 'TooOld': null } |
{ 'InsufficientFunds': { 'balance': bigint } };
export interface UpgradeArg { 'mode': [] | [Mode] }
export type Result = { 'Ok': SuccessWithFee } | { 'Err': ProtocolError }
export type Result_1 = { 'Ok': bigint } | { 'Err': ProtocolError }
export type OpenVaultResult = { 'Ok': OpenVaultSuccess } | { 'Err': ProtocolError }
export interface Vault {
    'owner': Principal,
    'vault_id': bigint,
    'borrowed_tal_amount': bigint,
    'ckbtc_margin_amount': bigint,
}
export interface VaultArg { 'vault_id': bigint, 'amount': bigint }
export interface _SERVICE {
    'add_margin_to_vault': ActorMethod<
        [VaultArg],
        { 'Ok': bigint } |
        { 'Err': ProtocolError }
    >,
    'borrow_from_vault': ActorMethod<
        [VaultArg],
        { 'Ok': SuccessWithFee } |
        { 'Err': ProtocolError }
    >,
    'claim_liquidity_returns': ActorMethod<
        [],
        { 'Ok': bigint } |
        { 'Err': ProtocolError }
    >,
    'close_vault': ActorMethod<
        [bigint],
        { 'Ok': [] | [bigint] } |
        { 'Err': ProtocolError }
    >,
    'get_events': ActorMethod<[GetEventsArg], Array<Event>>,
    'get_fees': ActorMethod<[bigint], Fees>,
    'get_liquidity_status': ActorMethod<[Principal], LiquidityStatus>,
    'get_protocol_status': ActorMethod<[], ProtocolStatus>,
    'get_vault_history': ActorMethod<[bigint], Array<Event>>,
    'get_vaults': ActorMethod<[[] | [Principal]], Array<Vault>>,
    'open_vault': ActorMethod<
        [bigint],
        { 'Ok': OpenVaultSuccess } |
        { 'Err': ProtocolError }
    >,
    'provide_liquidity': ActorMethod<
        [bigint],
        { 'Ok': bigint } |
        { 'Err': ProtocolError }
    >,
    'redeem_ckbtc': ActorMethod<
        [bigint],
        { 'Ok': SuccessWithFee } |
        { 'Err': ProtocolError }
    >,
    'repay_to_vault': ActorMethod<
        [VaultArg],
        { 'Ok': bigint } |
        { 'Err': ProtocolError }
    >,
    'withdraw_liquidity': ActorMethod<
        [bigint],
        { 'Ok': bigint } |
        { 'Err': ProtocolError }
    >,
}
