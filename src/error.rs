//! Error types for this crate.
use thiserror::Error;

use crate::{ClientId, TxId};

/// Any kind of error that can happen when processing a [crate::Transaction] in a [crate::Ledger].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
pub enum LedgerError {
    #[error("not enough funds available to run transaction")]
    NotEnoughFunds,
    #[error("unknown transaction with user '{0}', id '{1}'")]
    UnknownTx(ClientId, TxId),
    #[error("transaction has already been disputed")]
    AlreadyDisputed,
    #[error("transaction is not currently disputed")]
    NotDisputed,
    #[error("account is frozen")]
    FrozenAccount,
}

/// Any kind of error that can happen when deserializing a [crate::Transaction] value.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Error)]
pub enum ParseError {
    #[error("amount not provided")]
    MissingAmount,
    #[error("unknown transaction type '{0}'")]
    UnknownTx(String),
}
