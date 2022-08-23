//! Define all supported transactions.
use crate::core::{ClientId, TxAmount, TxId};

use serde::Deserialize;

/// A generic [Transaction].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "TransactionRecord")]
pub enum Transaction {
    Deposit(Deposit),
    Withdrawal(Withdrawal),
    Dispute(Dispute),
    Resolve(Resolve),
    Chargeback(Chargeback),
}

impl Transaction {
    /// Build a [csv::ReaderBuilder] configured to read a CSV formatted [Transaction] stream.
    pub fn configured_csv_reader_builder() -> csv::ReaderBuilder {
        let mut builder = csv::ReaderBuilder::new();
        builder
            // Expect header input
            .has_headers(true)
            // Allow whitespace
            .trim(csv::Trim::All)
            // Allow trailing fields to be omitted
            .flexible(true);
        builder
    }
}

// A type used to deserialize [Transaction] from an input CSV stream.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
struct TransactionRecord<'a> {
    #[serde(rename = "type")]
    type_: &'a str,
    client: ClientId,
    tx: TxId,
    amount: Option<TxAmount>,
}

impl TryFrom<TransactionRecord<'_>> for Transaction {
    // FIXME: use an actual error type.
    type Error = String;

    fn try_from(value: TransactionRecord<'_>) -> Result<Self, Self::Error> {
        let TransactionRecord {
            type_,
            client,
            tx,
            amount,
        } = value;

        let transaction = match type_ {
            "deposit" => {
                let amount = amount.ok_or("Missing amount for transaction")?;
                Transaction::Deposit(Deposit { client, tx, amount })
            }
            "withdrawal" => {
                let amount = amount.ok_or("Missing amount for transaction")?;
                Transaction::Withdrawal(Withdrawal { client, tx, amount })
            }
            "dispute" => Transaction::Dispute(Dispute { client, tx }),
            "resolve" => Transaction::Resolve(Resolve { client, tx }),
            "chargeback" => Transaction::Chargeback(Chargeback { client, tx }),
            _ => return Err(format!("Unkown transaction type '{}'", type_)),
        };
        Ok(transaction)
    }
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

#[cfg(test)]
mod test {
    use super::*;
    use fpdec::{Dec, Decimal};

    fn parse_transaction(input: &str) -> Transaction {
        let rdr = Transaction::configured_csv_reader_builder().from_reader(input.as_bytes());
        rdr.into_deserialize().next().unwrap().unwrap()
    }

    #[test]
    fn deserialize_deposit() {
        let data = "type,client,tx,amount\ndeposit,1,2,3.0";
        assert_eq!(
            parse_transaction(data),
            Transaction::Deposit(Deposit {
                client: ClientId(1),
                tx: TxId(2),
                amount: TxAmount(Dec!(3.0))
            }),
        );
    }

    #[test]
    fn deserialize_withdrawal() {
        let data = "type,client,tx,amount\nwithdrawal,1,2,3.0";
        assert_eq!(
            parse_transaction(data),
            Transaction::Withdrawal(Withdrawal {
                client: ClientId(1),
                tx: TxId(2),
                amount: TxAmount(Dec!(3.0))
            }),
        );
    }

    #[test]
    fn deserialize_dispute() {
        let data = "type,client,tx,amount\ndispute,1,2";
        assert_eq!(
            parse_transaction(data),
            Transaction::Dispute(Dispute {
                client: ClientId(1),
                tx: TxId(2),
            }),
        );
    }

    #[test]
    fn deserialize_resolve() {
        let data = "type,client,tx,amount\nresolve,1,2";
        assert_eq!(
            parse_transaction(data),
            Transaction::Resolve(Resolve {
                client: ClientId(1),
                tx: TxId(2),
            }),
        );
    }

    #[test]
    fn deserialize_chargeback() {
        let data = "type,client,tx,amount\nchargeback,1,2";
        assert_eq!(
            parse_transaction(data),
            Transaction::Chargeback(Chargeback {
                client: ClientId(1),
                tx: TxId(2),
            }),
        );
    }

    #[test]
    fn deserialize_transactions() {
        let data = concat!(
            "type,client,tx,amount\n",
            "deposit, 1, 2, 12.0000\n",
            "withdrawal    , 3,4, 42.27         \n",
            "dispute, 5   ,             6,    \n",
            "     resolve,7,8,\n",
            "chargeback,9,10",
        );
        let rdr = Transaction::configured_csv_reader_builder().from_reader(data.as_bytes());
        let transactions: Result<Vec<Transaction>, _> = rdr.into_deserialize().collect();
        assert_eq!(
            transactions.unwrap(),
            vec![
                Transaction::Deposit(Deposit {
                    client: ClientId(1),
                    tx: TxId(2),
                    amount: TxAmount(Dec!(12.0000)),
                }),
                Transaction::Withdrawal(Withdrawal {
                    client: ClientId(3),
                    tx: TxId(4),
                    amount: TxAmount(Dec!(42.27)),
                }),
                Transaction::Dispute(Dispute {
                    client: ClientId(5),
                    tx: TxId(6),
                }),
                Transaction::Resolve(Resolve {
                    client: ClientId(7),
                    tx: TxId(8),
                }),
                Transaction::Chargeback(Chargeback {
                    client: ClientId(9),
                    tx: TxId(10),
                }),
            ]
        );
    }
}
