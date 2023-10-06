export enum Asset {
    ckBTC,
    TAL
}

export const CKBTC_TRANSFER_FEE = 10;
export const TAL_TRANSFER_FEE = 1_000_000;
export const U64_MAX = BigInt(18_446_744_073_709_551_615);
export const E8S = 100_000_000;

export function display_principal(principal) {
    const a = principal.split('-')
    return a[0] + '...' + a[a.length - 1]
}


export function display_tal_e8s(amount: bigint): string {
    const E8S = 100_000_000;
    return (Number(amount) / E8S).toFixed(2);
}

export function display_ckbtc_e8s(amount: bigint): string {
    const E8S = 100_000_000;
    const result = Number(amount) / E8S;
    return result.toLocaleString(undefined, { minimumFractionDigits: 4, maximumFractionDigits: 4 });
}
