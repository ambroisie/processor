//! Core types used in the processing of payments.

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Clients are anonymous, identified by globally unique ids. "16-bit ought to be enough for
/// anyone".
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct ClientId(pub u16);

/// Transactions are identified by a globally unique id. 32 bit is sufficient for our puposes.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize,
)]
#[serde(transparent)]
pub struct TxId(pub u32);

/// Amounts are represented as exact decimals, up to four places past the decimal.
/// For ease of implementation, make use of [fpdec::Decimal] instead of implementing a custom
/// fixed-point number.
#[serde_as]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(transparent)]
pub struct TxAmount(#[serde_as(as = "DisplayFromStr")] pub fpdec::Decimal);
