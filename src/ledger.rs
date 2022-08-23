//! A ledger implementation to track all transactions.

use crate::{
    Chargeback, ClientId, Deposit, Dispute, LedgerError, Resolve, Transaction, TxAmount, TxId,
    Withdrawal,
};

/// A ledger of accounts, which processes transactions one at a time.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    accounts: std::collections::HashMap<ClientId, AccountInfo>,
    transaction_amounts: std::collections::HashMap<(ClientId, TxId), TxAmount>,
    transaction_state: std::collections::HashMap<(ClientId, TxId), TxState>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccountInfo {
    available_funds: TxAmount,
    held_funds: TxAmount,
    locked: bool,
}

/// Represent the state of a transaction. Here are the possible transitions:
///
/// ```graphviz
/// Processed -> Disputed
/// Disputed -> Resolved
/// Disputed -> ChargedBack
/// ```
///
/// The starting state is `Processed`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TxState {
    /// A transaction was just accepted.
    Processed,
    /// A transaction dispute has been processed.
    Disputed,
    /// A transaction disputed was resolved.
    Resolved,
    /// A transaction disputed was chargedback.
    ChargedBack,
}

impl TxState {
    pub fn apply_dispute(
        &mut self,
        account: &mut AccountInfo,
        amount: TxAmount,
    ) -> LedgerResult<()> {
        if *self != Self::Processed {
            return Err(LedgerError::AlreadyDisputed);
        }

        account.apply_dispute(amount)?;
        *self = Self::Disputed;
        Ok(())
    }

    pub fn apply_resolution(
        &mut self,
        account: &mut AccountInfo,
        amount: TxAmount,
    ) -> LedgerResult<()> {
        if *self != Self::Disputed {
            return Err(LedgerError::NotDisputed);
        }

        account.apply_resolution(amount)?;
        *self = Self::Resolved;
        Ok(())
    }

    pub fn apply_chargeback(
        &mut self,
        account: &mut AccountInfo,
        amount: TxAmount,
    ) -> LedgerResult<()> {
        if *self != Self::Disputed {
            return Err(LedgerError::NotDisputed);
        }

        account.apply_chargeback(amount)?;
        *self = Self::ChargedBack;
        Ok(())
    }
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
            Transaction::Dispute(tx) => self.dispute(tx),
            Transaction::Resolve(tx) => self.resolve(tx),
            Transaction::Chargeback(tx) => self.chargeback(tx),
        }
    }

    fn delta(&mut self, client: ClientId, tx: TxId, delta: TxAmount) -> LedgerResult<()> {
        let account = self.accounts.entry(client).or_default();
        account.apply_delta(delta)?;
        self.transaction_amounts.insert((client, tx), delta);
        self.transaction_state
            .insert((client, tx), TxState::Processed);
        Ok(())
    }

    fn dispute(&mut self, Dispute { client, tx }: Dispute) -> LedgerResult<()> {
        let (current_state, account, amount) = self.get_past_transaction_info(client, tx)?;
        current_state.apply_dispute(account, amount)
    }

    fn resolve(&mut self, Resolve { client, tx }: Resolve) -> LedgerResult<()> {
        let (current_state, account, amount) = self.get_past_transaction_info(client, tx)?;
        current_state.apply_resolution(account, amount)
    }

    fn chargeback(&mut self, Chargeback { client, tx }: Chargeback) -> LedgerResult<()> {
        let (current_state, account, amount) = self.get_past_transaction_info(client, tx)?;
        current_state.apply_chargeback(account, amount)
    }

    fn get_past_transaction_info(
        &mut self,
        client: ClientId,
        tx: TxId,
    ) -> LedgerResult<(&mut TxState, &mut AccountInfo, TxAmount)> {
        let current_state = self
            .transaction_state
            .get_mut(&(client, tx))
            .ok_or(LedgerError::UnknownTx(client, tx))?;
        let account = self
            .accounts
            .get_mut(&client)
            .expect("a processed transaction should have its account recorded");
        let amount = self
            .transaction_amounts
            .get(&(client, tx))
            .cloned()
            .expect("a processed transaction should have its amount recorded");
        Ok((current_state, account, amount))
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

    pub fn apply_delta(&mut self, delta: TxAmount) -> LedgerResult<()> {
        self.check_frozen()?;
        let new_balance = self.available_funds() + delta;
        if new_balance < TxAmount::ZERO {
            return Err(LedgerError::NotEnoughFunds);
        }
        self.available_funds = new_balance;
        Ok(())
    }

    pub fn apply_dispute(&mut self, delta: TxAmount) -> LedgerResult<()> {
        self.check_frozen()?;
        // FIXME: should we check for negative funds?
        self.available_funds -= delta;
        self.held_funds += delta;
        Ok(())
    }

    pub fn apply_resolution(&mut self, delta: TxAmount) -> LedgerResult<()> {
        self.check_frozen()?;
        // FIXME: should we check for negative funds?
        self.available_funds += delta;
        self.held_funds -= delta;
        Ok(())
    }

    pub fn apply_chargeback(&mut self, delta: TxAmount) -> LedgerResult<()> {
        self.check_frozen()?;
        // FIXME: should we check for negative funds?
        self.held_funds -= delta;
        self.locked = true;
        Ok(())
    }

    fn check_frozen(&self) -> LedgerResult<()> {
        if self.is_locked() {
            Err(LedgerError::FrozenAccount)
        } else {
            Ok(())
        }
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

    #[test]
    fn dispute_deposit() {
        let ledger = process_transactions(inline_csv!(
            "type,       client, tx, amount",
            "deposit,         1,  1,   1.0",
            "dispute,         1,  1",
        ))
        .unwrap();
        check_ledger(
            &ledger,
            expect![[r#"
                client,available,held,total,locked
                1,0.0,1.0,1.0,false
            "#]],
        );
    }

    #[test]
    fn dispute_withdrawal() {
        let ledger = process_transactions(inline_csv!(
            "type,       client, tx, amount",
            "deposit,         1,  1,   1.0",
            "withdrawal,      1,  2,   1.0",
            "dispute,         1,  2",
        ))
        .unwrap();
        check_ledger(
            &ledger,
            expect![[r#"
                client,available,held,total,locked
                1,1.0,-1.0,0.0,false
            "#]],
        );
    }

    #[test]
    fn resolve_dispute() {
        let ledger = process_transactions(inline_csv!(
            "type,       client, tx, amount",
            "deposit,         1,  1,   1.0",
            "dispute,         1,  1",
            "resolve,         1,  1",
        ))
        .unwrap();
        check_ledger(
            &ledger,
            expect![[r#"
                client,available,held,total,locked
                1,1.0,0.0,1.0,false
            "#]],
        );
    }

    #[test]
    fn chargeback_dispute() {
        let ledger = process_transactions(inline_csv!(
            "type,       client, tx, amount",
            "deposit,         1,  1,   1.0",
            "dispute,         1,  1",
            "chargeback,      1,  1",
        ))
        .unwrap();
        check_ledger(
            &ledger,
            expect![[r#"
                client,available,held,total,locked
                1,0.0,0.0,0.0,true
            "#]],
        );
    }
}
