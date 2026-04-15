use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;
use std::time;

use crate::wal::errors::WalError;
use crate::wal::log_entry::LogEntry;
use crate::wal::types::entry_type::EntryType;
use crate::wal::types::opration_type::OprationType;
use crate::wal::wal_header::WalHeader;

pub struct WalWriter {
    file: File,
    current_sequence: u64,
    file_offset: u64,
    start_sequence: u64,
}

impl WalWriter {
    pub fn new(path: &str) -> Result<Self, WalError> {
        let path = Path::new(path);
        
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| WalError::IOError(e))?;
            }
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .append(false)
            .open(path)
            .map_err(|e| WalError::IOError(e))?;

        let metadata = file.metadata().map_err(|e| WalError::IOError(e))?;
        let file_size = metadata.len();

        let (start_sequence, file_offset) = if file_size == 0 {
            let header = WalHeader::new(
                "arookieofcDB".to_string(),
                1,
                1,
                time::SystemTime::now()
                    .duration_since(time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64,
            );
            let header_bytes = header.to_bytes();
            file.write_all(&header_bytes)
                .map_err(|e| WalError::IOError(e))?;
            file.sync_all().map_err(|e| WalError::IOError(e))?;
            (1, header_bytes.len() as u64)
        } else {
            let mut header_bytes = [0u8; 48];
            file.read_exact(&mut header_bytes)
                .map_err(|e| WalError::IOError(e))?;
            let header = WalHeader::from_bytes(&header_bytes)?;
            (header.get_start_sequence(), file_size)
        };

        file.seek(SeekFrom::End(0))
            .map_err(|e| WalError::IOError(e))?;

        Ok(Self {
            file,
            current_sequence: start_sequence,
            file_offset,
            start_sequence,
        })
    }

    pub fn write_entry(&mut self, key: &[u8], value: &[u8]) -> Result<u64, WalError> {
        let seq = self.current_sequence;
        let timestamp = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let data = LogEntry::build_entry_data(seq, key, value, timestamp);
        let checksum = LogEntry::crc32(&data);

        let entry = LogEntry::new(
            seq,
            EntryType::Log,
            OprationType::Insert,
            key.to_vec(),
            value.to_vec(),
            checksum,
            timestamp,
        );

        let entry_bytes = entry.to_bytes();

        self.file
            .write_all(&entry_bytes)
            .map_err(|e| WalError::IOError(e))?;
        
        self.file
            .sync_all()
            .map_err(|e| WalError::IOError(e))?;

        self.current_sequence += 1;
        self.file_offset += entry_bytes.len() as u64;

        Ok(seq)
    }

    pub fn write_checkpoint(&mut self, seq: u64) -> Result<u64, WalError> {
        let timestamp = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let data = LogEntry::build_entry_data(seq, b"", b"", timestamp);
        let checksum = LogEntry::crc32(&data);

        let entry = LogEntry::new(
            seq,
            EntryType::Checkpoint,
            OprationType::Insert,
            Vec::new(),
            Vec::new(),
            checksum,
            timestamp,
        );

        let entry_bytes = entry.to_bytes();

        self.file
            .write_all(&entry_bytes)
            .map_err(|e| WalError::IOError(e))?;
        
        self.file
            .sync_all()
            .map_err(|e| WalError::IOError(e))?;

        Ok(seq)
    }

    pub fn current_sequence(&self) -> u64 {
        self.current_sequence
    }

    pub fn start_sequence(&self) -> u64 {
        self.start_sequence
    }

    pub fn file_offset(&self) -> u64 {
        self.file_offset
    }
}
