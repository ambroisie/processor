//! A ledger implementation to track all transactions.

use crate::{ClientId, Deposit, LedgerError, Transaction, TxAmount, TxId, Withdrawal};

/// A ledger of accounts, which processes transactions one at a time.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    accounts: std::collections::HashMap<ClientId, AccountInfo>,
    reversible_transactions: std::collections::HashMap<TxId, (ClientId, TxAmount)>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountInfo {
    available_funds: TxAmount,
    held_funds: TxAmount,
    locked: bool,
}

type LedgerResult<T> = Result<T, LedgerError>;

impl Ledger {
    pub fn new() -> Self {
        Default::default()
    }

    /// Serialize a [Ledger] to CSV.
    pub fn dump_csv<W: std::io::Write>(&self, writer: &mut csv::Writer<W>) -> csv::Result<()> {
        // Keep list of accounts ordered for easier diffs
        let ordered_accounts: std::collections::BTreeMap<_, _> = self.accounts.iter().collect();
        writer.write_record(&["client", "available", "held", "total", "locked"])?;
        for (id, info) in ordered_accounts.into_iter() {
            writer.write_record(&[
                id.0.to_string(),
                info.available_funds().0.to_string(),
                info.held_funds().0.to_string(),
                info.total_funds().0.to_string(),
                info.is_locked().to_string(),
            ])?
        }
        Ok(())
    }

    pub fn process(&mut self, tx: Transaction) -> LedgerResult<()> {
        match tx {
            Transaction::Deposit(Deposit { client, tx, amount }) => self.delta(client, tx, amount),
            Transaction::Withdrawal(Withdrawal { client, tx, amount }) => {
                self.delta(client, tx, -amount)
            }
            _ => todo!(),
        }
    }

    fn delta(&mut self, client: ClientId, tx: TxId, delta: TxAmount) -> LedgerResult<()> {
        let account = self.accounts.entry(client).or_default();
        let new_balance = account.available_funds() + delta;
        if new_balance < TxAmount::ZERO {
            return Err(LedgerError::NotEnoughFunds);
        }
        account.available_funds = new_balance;
        self.reversible_transactions.insert(tx, (client, delta));
        Ok(())
    }
}

impl AccountInfo {
    /// Whether or not an account has been locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// The funds that are usable on this account.
    pub fn available_funds(&self) -> TxAmount {
        self.available_funds
    }

    /// The funds that have been locked pending resolution of dispute.
    pub fn held_funds(&self) -> TxAmount {
        self.held_funds
    }

    /// The totals funds on an account, i.e: available funds and held funds.
    pub fn total_funds(&self) -> TxAmount {
        self.available_funds + self.held_funds
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use expect_test::{expect, Expect};

    macro_rules! inline_csv {
        ($line:literal) => {
            $line
        };
        ($line:literal, $($lines:literal),+ $(,)?) => {
            concat!($line, "\n", inline_csv!($($lines),+))
        };
    }

    fn process_transactions(input: &str) -> Result<Ledger, LedgerError> {
        let mut ledger = Ledger::new();
        for tx in Transaction::configured_csv_reader_builder()
            .from_reader(input.as_bytes())
            .into_deserialize()
        {
            ledger.process(tx.unwrap())?
        }
        Ok(ledger)
    }

    fn check_ledger(ledger: &Ledger, expect: Expect) {
        let mut writer = csv::Writer::from_writer(vec![]);
        ledger.dump_csv(&mut writer).unwrap();
        let actual = String::from_utf8(writer.into_inner().unwrap()).unwrap();
        expect.assert_eq(&actual);
    }

    #[test]
    fn deposit_single_account() {
        let ledger = process_transactions(inline_csv!(
            "type,       client, tx, amount",
            "deposit,         1,  1,   1.0",
            "deposit,         1,  2,   2.0",
        ))
        .unwrap();
        check_ledger(
            &ledger,
            expect![[r#"
                client,available,held,total,locked
                1,3.0,0,3.0,false
            "#]],
        );
    }

    #[test]
    fn deposit_multiple_accounts() {
        let ledger = process_transactions(inline_csv!(
            "type,       client, tx, amount",
            "deposit,         1,  1,   1.0",
            "deposit,         2,  2,   1.0",
            "deposit,         1,  3,   2.0",
        ))
        .unwrap();
        check_ledger(
            &ledger,
            expect![[r#"
                client,available,held,total,locked
                1,3.0,0,3.0,false
                2,1.0,0,1.0,false
            "#]],
        );
    }

    #[test]
    fn deposit_and_withdrawal() {
        let ledger = process_transactions(inline_csv!(
            "type,       client, tx, amount",
            "deposit,         1,  1,   1.0",
            "deposit,         2,  2,   1.0",
            "deposit,         1,  3,   2.0",
            "withdrawal,      1,  4,   1.5",
            "withdrawal,      2,  5,   1.0",
        ))
        .unwrap();
        check_ledger(
            &ledger,
            expect![[r#"
                client,available,held,total,locked
                1,1.5,0,1.5,false
                2,0.0,0,0.0,false
            "#]],
        );
    }

    #[test]
    fn deposit_and_withdrawal_not_enough_funds() {
        let error = process_transactions(inline_csv!(
            "type,       client, tx, amount",
            "deposit,         2,  2,   1.0",
            "withdrawal,      2,  5,   3.0",
        ))
        .unwrap_err();
        assert_eq!(error, LedgerError::NotEnoughFunds);
    }
}
