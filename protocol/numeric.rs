use candid::types::TypeInner;
use candid::{CandidType, Deserialize, Nat};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{de::Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt;
use std::iter::Sum;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

#[cfg(test)]
mod tests;

const E8S: u64 = 100_000_000;
const TAL_DEC: u64 = 100_000_000;

#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Clone, Copy)]
pub struct Amount<T>(pub Decimal, pub PhantomData<T>);

impl<T> Serialize for Amount<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0.serialize())
    }
}

impl<'de, T> Deserialize<'de> for Amount<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let array: [u8; 16] = Deserialize::deserialize(deserializer)?;
        Ok(Amount(Decimal::deserialize(array), PhantomData))
    }
}

impl<T> CandidType for Amount<T> {
    fn _ty() -> candid::types::Type {
        TypeInner::Vec(TypeInner::Nat8.into()).into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_blob(&self.clone().to_array())
    }
}

impl<T> Amount<T> {
    pub fn to_f64(self) -> f64 {
        self.0.to_f64().unwrap()
    }

    pub fn to_array(&self) -> [u8; 16] {
        self.0.serialize()
    }
}

#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Clone, Copy)]
pub struct Token<T>(pub u64, pub PhantomData<T>);

impl<T> Serialize for Token<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.0)
    }
}

impl<'de, T> Deserialize<'de> for Token<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: u64 = Deserialize::deserialize(deserializer)?;
        Ok(Token(value, PhantomData))
    }
}

impl<T> CandidType for Token<T> {
    fn _ty() -> candid::types::Type {
        candid::types::TypeInner::Nat64.into()
    }

    fn idl_serialize<S>(&self, serializer: S) -> Result<(), S::Error>
    where
        S: candid::types::Serializer,
    {
        serializer.serialize_nat64(self.0)
    }
}

impl<T> Token<T> {
    pub fn to_u64(self) -> u64 {
        self.0
    }

    pub fn to_nat(self) -> Nat {
        Nat::from(self.0)
    }

    pub fn saturating_sub(self, other: Token<T>) -> Token<T> {
        if other.0 > self.0 {
            return Token::<T>(0, PhantomData::<T>);
        }
        Token::<T>(self.0 - other.0, PhantomData::<T>)
    }
}

impl<T> PartialOrd<u64> for Token<T> {
    fn partial_cmp(&self, &other: &u64) -> Option<Ordering> {
        self.0.partial_cmp(&other)
    }
}

impl<T> PartialEq<u64> for Token<T> {
    fn eq(&self, &other: &u64) -> bool {
        self.0 == other
    }
}

impl<T> PartialEq<Token<T>> for u64 {
    fn eq(&self, other: &Token<T>) -> bool {
        *self == other.0
    }
}

#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Serialize, Deserialize, Clone, Copy)]
pub enum TALEnum {}

#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Serialize, Deserialize, Clone, Copy)]
pub enum CKBTCEnum {}

#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Serialize, Deserialize, Clone, Copy)]
pub enum UsdBtcEnum {}

#[derive(PartialEq, Eq, Debug, Ord, PartialOrd, Serialize, Deserialize, Clone, Copy)]
pub enum RatioEnum {}

pub type TAL = Token<TALEnum>;
pub type CKBTC = Token<CKBTCEnum>;
pub type Ratio = Amount<RatioEnum>;
pub type UsdBtc = Amount<UsdBtcEnum>;

impl<T> Sum for Token<T> {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Token<T>>,
    {
        iter.fold(Token::<T>(0, PhantomData::<T>), |acc, x| acc + x)
    }
}

impl<T> Sub for Token<T> {
    type Output = Token<T>;

    fn sub(self, rhs: Token<T>) -> Token<T> {
        if rhs.0 > self.0 {
            panic!("underflow")
        }
        Token(self.0 - rhs.0, PhantomData)
    }
}

impl Sub for Ratio {
    type Output = Ratio;

    fn sub(self, rhs: Ratio) -> Self::Output {
        if rhs.0 > self.0 {
            panic!("underflow")
        }
        Ratio::from(self.0 - rhs.0)
    }
}

impl<T> Add for Token<T> {
    type Output = Token<T>;

    fn add(self, rhs: Token<T>) -> Token<T> {
        Token(self.0 + rhs.0, PhantomData)
    }
}

impl<T> Add for Amount<T> {
    type Output = Amount<T>;

    fn add(self, rhs: Amount<T>) -> Amount<T> {
        Amount(self.0 + rhs.0, PhantomData)
    }
}

impl From<u64> for CKBTC {
    fn from(value: u64) -> Self {
        Token(value, PhantomData::<CKBTCEnum>)
    }
}

impl From<u64> for TAL {
    fn from(value: u64) -> Self {
        Token(value, PhantomData::<TALEnum>)
    }
}

impl TAL {
    pub const fn new(value: u64) -> Self {
        Token(value, PhantomData::<TALEnum>)
    }
}

impl CKBTC {
    pub const fn new(value: u64) -> Self {
        Token(value, PhantomData::<CKBTCEnum>)
    }
}

impl Ratio {
    pub const fn new(value: Decimal) -> Self {
        Amount(value, PhantomData::<RatioEnum>)
    }

    pub fn pow(self, rhs: u64) -> Self {
        if rhs == 0 {
            return Amount(Decimal::ONE, PhantomData::<RatioEnum>);
        }
        let mut result = Decimal::ONE;
        for _ in 0..rhs {
            result *= self.0;
        }
        Amount(result, PhantomData::<RatioEnum>)
    }
}

impl UsdBtc {
    pub const fn new(value: Decimal) -> Self {
        Amount(value, PhantomData::<UsdBtcEnum>)
    }

    pub fn to_e8s(self) -> u64 {
        (self.0 * dec!(100_000_000)).to_u64().unwrap()
    }

    pub fn serialize(self) -> [u8; 16] {
        self.0.serialize()
    }

    pub fn deserialize(array: [u8; 16]) -> Self {
        UsdBtc::new(Decimal::deserialize(array))
    }
}

impl From<Decimal> for UsdBtc {
    fn from(value: Decimal) -> Self {
        Amount(value, PhantomData::<UsdBtcEnum>)
    }
}

impl From<Decimal> for Ratio {
    fn from(value: Decimal) -> Self {
        Amount(value, PhantomData::<RatioEnum>)
    }
}

impl Mul<UsdBtc> for CKBTC {
    type Output = TAL;

    fn mul(self, other: UsdBtc) -> TAL {
        let ckbtc_dec = Decimal::from_u64(self.0).expect("failed to construct decimal from u64")
            / dec!(100_000_000);
        let result = ckbtc_dec * other.0;
        let result_e8s = result * dec!(100_000_000);
        Token(
            result_e8s.to_u64().expect("failed to cast decimal as u64"),
            PhantomData::<TALEnum>,
        )
    }
}

impl<T> Mul<Ratio> for Token<T> {
    type Output = Token<T>;

    fn mul(self, other: Ratio) -> Token<T> {
        let ckbtc_dec = Decimal::from_u64(self.0).expect("failed to construct decimal from u64")
            / dec!(100_000_000);
        let result = ckbtc_dec * other.0;
        let result_e8s = result * dec!(100_000_000);
        Token(
            result_e8s.to_u64().expect("failed to cast decimal as u64"),
            PhantomData::<T>,
        )
    }
}

impl<T> AddAssign for Token<T> {
    fn add_assign(&mut self, rhs: Token<T>) {
        self.0 += rhs.0;
    }
}

impl<T> SubAssign for Token<T> {
    fn sub_assign(&mut self, rhs: Token<T>) {
        assert!(self.0 >= rhs.0);
        self.0 -= rhs.0;
    }
}

impl Mul<Ratio> for Ratio {
    type Output = Ratio;

    fn mul(self, other: Ratio) -> Ratio {
        let result = self.0 * other.0;
        Amount(result, PhantomData::<RatioEnum>)
    }
}

impl Div<UsdBtc> for TAL {
    type Output = CKBTC;

    fn div(self, other: UsdBtc) -> CKBTC {
        assert_ne!(other.0, Decimal::ZERO, "cannot divide {} by 0", self.0);
        let tal_dec = Decimal::from_u64(self.0).unwrap() / Decimal::from_u64(TAL_DEC).unwrap();
        let result = (tal_dec / other.0) * Decimal::from_u64(TAL_DEC).unwrap();
        let result_e8s = result.to_u64().unwrap();
        Token::<CKBTCEnum>(result_e8s, PhantomData::<CKBTCEnum>)
    }
}

impl Div<TAL> for TAL {
    type Output = Ratio;

    fn div(self, other: TAL) -> Ratio {
        assert_ne!(other.0, 0, "cannot divide {} by 0", self.0);
        let tal_dec = Decimal::from_u64(self.0).unwrap();
        let div_by = Decimal::from_u64(other.0).unwrap();
        let result = tal_dec / div_by;
        Amount::<RatioEnum>(result, PhantomData::<RatioEnum>)
    }
}

impl Div<Ratio> for TAL {
    type Output = TAL;

    fn div(self, other: Ratio) -> TAL {
        assert_ne!(other.0, Decimal::ZERO, "cannot divide {} by 0", self.0);
        let tal_dec = Decimal::from_u64(self.0).unwrap() / Decimal::from_u64(TAL_DEC).unwrap();
        let result = (tal_dec / other.0) * Decimal::from_u64(TAL_DEC).unwrap();
        let result_e8s = result.to_u64().unwrap();
        Token::<TALEnum>(result_e8s, PhantomData::<TALEnum>)
    }
}

impl Div<Ratio> for UsdBtc {
    type Output = UsdBtc;

    fn div(self, other: Ratio) -> UsdBtc {
        assert_ne!(other.0, Decimal::ZERO, "cannot divide {} by 0", self.0);
        Amount::<UsdBtcEnum>(self.0 / other.0, PhantomData::<UsdBtcEnum>)
    }
}

impl Div<CKBTC> for CKBTC {
    type Output = Ratio;

    fn div(self, other: CKBTC) -> Ratio {
        assert_ne!(other.0, 0, "cannot divide {} by 0", self.0);
        let tal_dec = Decimal::from_u64(self.0).unwrap();
        let div_by = Decimal::from_u64(other.0).unwrap();
        let result = tal_dec / div_by;
        Amount::<RatioEnum>(result, PhantomData::<RatioEnum>)
    }
}

impl<T> fmt::Display for Token<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let int = self.0 / E8S;
        let frac = self.0 % E8S;

        if frac > 0 {
            let frac_width: usize = {
                // Count decimal digits in the fraction part.
                let mut d = 0;
                let mut x = frac;
                while x > 0 {
                    d += 1;
                    x /= 10;
                }
                d
            };
            debug_assert!(frac_width <= 8);
            let frac_prefix: u64 = {
                // The fraction part without trailing zeros.
                let mut f = frac;
                while f % 10 == 0 {
                    f /= 10
                }
                f
            };

            write!(fmt, "{}.", int)?;
            for _ in 0..(8 - frac_width) {
                write!(fmt, "0")?;
            }
            write!(fmt, "{}", frac_prefix)
        } else {
            write!(fmt, "{}.0", int)
        }
    }
}

impl<T> fmt::Display for Amount<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.0)
    }
}
