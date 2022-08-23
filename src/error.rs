//! Error types for this crate.
use thiserror::Error;

/// Any kind of error that can happen when deserializing a [crate::Transaction] value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
pub enum ParseError {
    #[error("amount not provided")]
    MissingAmount,
    #[error("unknown transaction type '{0}'")]
    UnknownTx(String),
}
