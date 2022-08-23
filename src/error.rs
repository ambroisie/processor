//! Error types for this crate.
use thiserror::Error;

/// Any kind of error that can happen when processing a [crate::Transaction] in a [crate::Ledger].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
pub enum LedgerError {
    #[error("not enough funds available to run transaction")]
    NotEnoughFunds,
}

/// Any kind of error that can happen when deserializing a [crate::Transaction] value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
pub enum ParseError {
    #[error("amount not provided")]
    MissingAmount,
    #[error("unknown transaction type '{0}'")]
    UnknownTx(String),
}
