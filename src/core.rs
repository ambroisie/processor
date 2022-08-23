//! Core types used in the processing of payments.

use fpdec::{Dec, Decimal};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Clients are anonymous, identified by globally unique ids. "16-bit ought to be enough for
/// anyone".
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct ClientId(pub u16);

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Transactions are identified by a globally unique id. 32 bit is sufficient for our puposes.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct TxId(pub u32);

impl std::fmt::Display for TxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Amounts are represented as exact decimals, up to four places past the decimal.
/// For ease of implementation, make use of [fpdec::Decimal] instead of implementing a custom
/// fixed-point number.
#[serde_as]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TxAmount(#[serde_as(as = "DisplayFromStr")] pub Decimal);

impl TxAmount {
    pub const ZERO: Self = Self(Dec!(0));
}

impl std::fmt::Display for TxAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Add<TxAmount> for TxAmount {
    type Output = Self;

    fn add(self, rhs: TxAmount) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign<TxAmount> for TxAmount {
    fn add_assign(&mut self, rhs: TxAmount) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub<TxAmount> for TxAmount {
    type Output = Self;

    fn sub(self, rhs: TxAmount) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::SubAssign<TxAmount> for TxAmount {
    fn sub_assign(&mut self, rhs: TxAmount) {
        *self = *self - rhs;
    }
}

impl std::ops::Neg for TxAmount {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}
