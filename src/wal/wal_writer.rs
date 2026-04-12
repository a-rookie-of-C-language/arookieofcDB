use std::fs::File;
use std::io::Write;
use crate::wal::wal_header::WalHeader;
use crate::wal::errors::WalError;
use macros::all_args_constructor;

#[all_args_constructor]
pub struct WalWriter {
    file: File,
    current_sequence: u64,
    file_offset: u64,
    start_sequence: u64,
}

impl WalWriter {
    pub fn write_header(&mut self, header: &WalHeader) -> Result<(), WalError> {
        let header = header.to_bytes();
        self.file.write_all(&header)
          .map_err(|e| WalError::IOError(e))
    }
}