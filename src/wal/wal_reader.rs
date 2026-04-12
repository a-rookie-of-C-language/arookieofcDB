use std::fs::File;
use std::io::Read;
use crate::wal::wal_header::WalHeader;
use crate::wal::errors::WalError;
use macros::all_args_constructor;

#[all_args_constructor]
pub struct WalReader {
    file: File,
    current_sequence: u64,
    file_offset: u64,
    start_sequence: u64,
}

impl WalReader {

    pub fn read_header(&mut self) -> Result<WalHeader, WalError> {
        let header = WalHeader::new("arookieofcDB".to_string(), 1, 0, 0);
        self.file.read_exact(&mut header.to_bytes())
        .map_err(|e| WalError::IOError(e))?;
        Ok(header)
    }
}
