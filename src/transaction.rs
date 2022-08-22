//! Define all supported transactions.
use crate::core::{ClientId, TxAmount, TxId};

/// A generic [Transaction].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Transaction {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    Chargeback(Chargeback),
}

/// Deposit funds into an account, i.e: increase its balance by the amount given.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deposit {
    pub client: ClientId,
    pub tx: TxId,
    pub amount: TxAmount,
}

/// Withdraw funds from an account, i.e: the opposite of a [Deposit]. It is not allowed to withdraw
/// more than is available on the given account, and should result in a no-op.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Withdrawal {
    pub client: ClientId,
    pub tx: TxId,
    pub amount: TxAmount,
}

/// Hold funds for an erroneous transaction that should be reversed. Extract the amount of funds
/// corresponding to the given transaction into a held funds envelop by transfering it from their
/// available funds. If the given transaction does not exist, this results in a no-op.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dispute {
    pub client: ClientId,
    pub tx: TxId,
}

/// Resolve a [Dispute] in favor of the client: move the held funds for the diputed transaction
/// back to the available funds. If either the given transaction does not exist, or is not
/// disputed, this results in a no-op.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Resolve {
    pub client: ClientId,
    pub tx: TxId,
}

/// Resolve [Dispute] by withdrawing held funds. The held funds are decreased by the amount of the
/// transaction. An account which succesffully executed a chargeback is subsequently frozen. If
/// either the transaction does not exist, or is not disputed, this results in a no-op and the
/// account is *not* frozen.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Chargeback {
    pub client: ClientId,
    pub tx: TxId,
}
