//! Core types used in the processing of payments.

/// Clients are anonymous, identified by globally unique ids. "16-bit ought to be enough for
/// anyone".
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(pub u16);

/// Transactions are identified by a globally unique id. 32 bit is sufficient for our puposes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TxId(pub u32);

/// Amounts are represented as exact decimals, up to four places past the decimal.
/// For ease of implementation, make use of [fpdec::Decimal] instead of implementing a custom
/// fixed-point number.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct TxAmount(pub fpdec::Decimal);
