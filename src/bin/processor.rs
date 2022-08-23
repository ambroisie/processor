use thiserror::Error;

use processor::{Ledger, Transaction};

/// Any kind of error in the pipeline CSV parsing -> payment processing -> final state output.
#[derive(Debug, Error)]
pub enum Error {
    #[error("missing input file argument")]
    MissingFile,
    #[error("error during CSV processing: {0}")]
    CsvError(#[from] csv::Error),
}

fn main() -> Result<(), Error> {
    let mut ledger = Ledger::new();

    let path = std::env::args_os()
        // Skip argv[0]
        .skip(1)
        // Expect a file name here
        .next()
        .ok_or(Error::MissingFile)?;

    for (tx, index) in Transaction::configured_csv_reader_builder()
        .from_path(path)?
        .into_deserialize()
        .zip(1..)
    {
        match ledger.process(tx?) {
            // All errors are logged but should not stop processing
            Err(err) => eprintln!("error during processing: transaction {}: {}", index, err),
            _ => {}
        }
    }

    let mut writer = csv::Writer::from_writer(std::io::stdout());
    ledger.dump_csv(&mut writer)?;

    Ok(())
}
