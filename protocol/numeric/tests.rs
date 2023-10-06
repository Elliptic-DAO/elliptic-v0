use crate::numeric::{Ratio, UsdBtc, CKBTC, E8S, TAL};
use rust_decimal_macros::dec;

#[test]
fn ckbtc_mul_by_usdbtc_available() {
    let ckbtc_token: CKBTC = 100_000_000_u64.into();
    let rate_amount: UsdBtc = dec!(20_000.0).into();
    assert_eq!(ckbtc_token * rate_amount, TAL::from(20_000 * E8S));
}

#[test]
fn ckbtc_mul_by_ratio() {
    let ckbtc_token: CKBTC = 100_000_000_u64.into();
    let ratio_amount: Ratio = dec!(0.5).into();
    assert_eq!(ckbtc_token * ratio_amount, CKBTC::from(50_000_000));
}

#[test]
fn ckbtc_mul_by_0() {
    let ckbtc_token: CKBTC = 100_000_000_u64.into();
    let ratio_amount: Ratio = dec!(0.0).into();
    assert_eq!(ckbtc_token * ratio_amount, CKBTC::from(0));
}

#[test]
fn ratio_mul_by_0() {
    let ckbtc_token: CKBTC = 0_u64.into();
    let ratio_amount: Ratio = dec!(1.0).into();
    assert_eq!(ckbtc_token * ratio_amount, CKBTC::from(0));
}

#[test]
fn tal_mul_by_ratio() {
    let tal_token: TAL = 100_u64.into();
    let ratio: Ratio = dec!(0.5).into();
    assert_eq!(tal_token * ratio, TAL::from(50_u64));
}

#[test]
fn tal_div_by_usdbtc() {
    let rate: UsdBtc = dec!(1000).into();
    let tal: TAL = (100 * 100_000_000).into();
    let result = tal / rate;
    assert_eq!(CKBTC::from(10_000_000), result);
}
