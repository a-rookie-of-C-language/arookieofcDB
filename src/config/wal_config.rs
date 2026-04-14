use macros::bean;

use crate::wal::wal_writer::WalWriter;

#[bean(name = "wal_writer")]
fn wal_writer() -> WalWriter {
    match WalWriter::new("wal.db") {
        Ok(writer) => writer,
        Err(e) => panic!("Failed to create wal writer: {:?}", e),
    }
}