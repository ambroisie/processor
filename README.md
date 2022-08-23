# Payment processor

## Implementation details

### Strong typing

The `crate::core` module contain three new-type-style tuple-struct definitions:
`ClientId`, `TxId`, and `TxAmount`. The reason for new-typing instead of using
bare `u16` or `u32` values is to avoid potentially mixing up different kinds of
ids.

### Monetary values

The implementation for `TxAmount` makes use of a `fpdec::Dec` instead of
a floating point value: using floats for financial values is a terrible idea,
instead we should use integers to represent exact decimal fractions.
`fpdec::Dec` was used instead of a custom implementation for convenience. It
does allow storing values with a precision that is way higher than is needed in
our case (input/output should be limited 4 digits after the decimal point) but
this does not affect us negatively. It would be easy to swap in a custom numeric
type if it turns out to be more appropriate for our needs down the line.

### Serialization/deserialization

Unfortunately, most of the code relating to (de)serialization is less than
ideal, from a code aesthetic view-point, under-using the idiomatic `serde`
library. Since CSV as an input/output format is very limited when paired with
`serde`, either some hacks were needed to massage the input into the correct
format for consumption by the payment processor (see
`crate::transaction::TransactionRecord`) or for output (see
`crate::Ledger::dump_csv`). The reasons are outlined in the related commits for
these features (see [1], [2], [3] for upstream issues).

Another thing to note: `Ledger::dump_csv` outputs the accounts in order (even
though they are stored unordered), to simplify diff-ing and testing.

[1]: https://github.com/BurntSushi/rust-csv/issues/211
[2]: https://github.com/BurntSushi/rust-csv/issues/172
[3]: https://github.com/BurntSushi/rust-csv/issues/98

### Account information

Accounts are represented as a map from `ClientId` to `AccountInfo`. The funds
are split between `held_funds` and `available_funds` (relating to disputes). The
total funds are computed when needed but otherwise not stored to avoid
dis-synchronization between a potential `total_funds` and its `held_funds
+ available_funds` equivalent.

Once a client account has been frozen (after a `Chargeback` transaction) then
any attempt to modify the balance of this account will result in an error.

### Transaction log

It is assumed that each transaction id is unique, however the key to map into
transaction-related data is `(ClientId, TxId)`. This is done to simplify the
error-handling in case a valid transaction is used with an invalid user and
vice-versa. Since all of `dispute`, `resolve`, and `chargeback` reference both
ids together, we should check that both of them are correct before further
processing.

### Testing

All behaviour testing was done using unit tests runnable with `cargo test`.
`expect-test` (of `rust-analyzer` fame) was used to easily compare between the
final state of a ledger and its expected value, and for the ease of writing new
tests (or updating them, should a bug be found and squashed).

### Error handling

All errors that are raised from `Ledger::process` are non-fatal. They are the
result of invalid input, and mean that the transaction's processing was stopped,
and it is ignored.

The few errors states that could arise, from internal invariants not being
upheld, are instead surfaced through panics as further processing cannot go on
once an invariant is broken. The few `expect` calls inside
`Ledger::get_past_transaction_info` are used for this reason.

### Parallelisation

Currently, the code is single threaded, reading the input CSV in a streaming
fashion and processing transactions one-at-a-time. If we wanted to expose the
ledger in a way that allows multiple transactions to transpire at once (e.g:
a REST API), we would need to guard data accesses to ensure that no invariants
are broken. Similarly, TOCTOU issues would be a big deal (for example, trying to
`chargeback` the same `dispute`d transaction twice at the same time: the first
one could finish processing after the second one already checked that it was
initially in `dispute` state but before applying the chargeback, resulting in
a double charge). To deal with that we could serialize all accesses with a big
`Mutex`, but this would be slow and no different from handling each transaction
serially. Or we could instead have a lock per account affected by a transaction
which would be obtained at the very beginning of that transaction's processing
before any other action is taken. This would allow transactions affecting
different accounts to be processed in parallel, and would not impact the
correctness of the ledger or affect its internal invariants. This would be done
thanks to a concurrent hash-map with fine-grained locking (either bucket-level
or entry-level).

### Disputes

It is unclear whether `dispute` can be applied to both `deposit` and `withdraw`
transactions, or only the later. I took the more general position that it should
be possible to do both, but the point could be made that, rationally, who in
their right minds would dispute against some unexpected funds showing up in
their accounts (it's free money!).

Handling disputes is otherwise a very simple state machine, so I implemented it
using the `TxState` enum. This meant that any of `dispute`, `resolve`, and
`chargeback` are handled very similarly, and resulted in (IMHO) quite elegant
code to guard against illegal states (like trying to dispute a transaction that
was already disputed).
