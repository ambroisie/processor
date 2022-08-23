//! A ledger implementation to track all transactions.

use crate::{ClientId, TxAmount, TxId};

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
