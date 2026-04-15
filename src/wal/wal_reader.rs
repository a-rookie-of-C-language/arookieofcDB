use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::wal::errors::WalError;
use crate::wal::log_entry::LogEntry;
use crate::wal::wal_header::WalHeader;

const HEADER_SIZE: usize = 48;

pub struct WalReader {
    file: File,
    current_sequence: u64,
    file_offset: u64,
    start_sequence: u64,
    file_size: u64,
}

impl WalReader {
    pub fn new(path: &str) -> Result<Self, WalError> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(WalError::FileNotFound);
        }

        let mut file = File::open(path).map_err(|e| WalError::IOError(e))?;
        let metadata = file.metadata().map_err(|e| WalError::IOError(e))?;
        let file_size = metadata.len();

        if file_size < HEADER_SIZE as u64 {
            return Err(WalError::InvalidHeader("file too short".to_string()));
        }

        let mut header_bytes = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_bytes)
            .map_err(|e| WalError::IOError(e))?;
        let header = WalHeader::from_bytes(&header_bytes)?;

        Ok(Self {
            file,
            current_sequence: header.get_start_sequence(),
            file_offset: HEADER_SIZE as u64,
            start_sequence: header.get_start_sequence(),
            file_size,
        })
    }

    pub fn read_header(&mut self) -> Result<WalHeader, WalError> {
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|e| WalError::IOError(e))?;

        let mut header_bytes = [0u8; HEADER_SIZE];
        self.file.read_exact(&mut header_bytes)
            .map_err(|e| WalError::IOError(e))?;
        let header = WalHeader::from_bytes(&header_bytes)?;

        self.file
            .seek(SeekFrom::Start(self.file_offset))
            .map_err(|e| WalError::IOError(e))?;

        Ok(header)
    }

    pub fn read_entry(&mut self) -> Result<Option<LogEntry>, WalError> {
        if self.file_offset >= self.file_size {
            return Ok(None);
        }

        let remaining = (self.file_size - self.file_offset) as usize;
        if remaining < 33 {
            return Ok(None);
        }

        let mut key_len_bytes = [0u8; 4];
        self.file
            .seek(SeekFrom::Start(self.file_offset + 10))
            .map_err(|e| WalError::IOError(e))?;
        self.file.read_exact(&mut key_len_bytes)
            .map_err(|e| WalError::IOError(e))?;
        let key_len = u32::from_be_bytes(key_len_bytes);

        self.file
            .seek(SeekFrom::Start(self.file_offset + 14 + key_len as u64))
            .map_err(|e| WalError::IOError(e))?;
        let mut value_len_bytes = [0u8; 4];
        self.file.read_exact(&mut value_len_bytes)
            .map_err(|e| WalError::IOError(e))?;
        let value_len = u32::from_be_bytes(value_len_bytes);

        let entry_size = LogEntry::entry_size(key_len as usize, value_len as usize);
        let entry_bytes = (self.file_offset + entry_size as u64).min(self.file_size);
        let actual_size = (entry_bytes - self.file_offset) as usize;

        if actual_size < entry_size {
            return Ok(None);
        }

        self.file
            .seek(SeekFrom::Start(self.file_offset))
            .map_err(|e| WalError::IOError(e))?;

        let mut buffer = vec![0u8; entry_size];
        self.file.read_exact(&mut buffer)
            .map_err(|e| WalError::IOError(e))?;

        let entry = LogEntry::from_bytes(&buffer)?;

        if !entry.verify_checksum() {
            return Err(WalError::ChecksumMismatch);
        }

        self.current_sequence = entry.seq;
        self.file_offset += entry_size as u64;

        Ok(Some(entry))
    }

    pub fn seek_to(&mut self, sequence: u64) -> Result<(), WalError> {
        if sequence < self.start_sequence {
            return Err(WalError::InvalidHeader(
                format!("sequence {} before start sequence {}", sequence, self.start_sequence)
            ));
        }

        self.file
            .seek(SeekFrom::Start(HEADER_SIZE as u64))
            .map_err(|e| WalError::IOError(e))?;
        self.file_offset = HEADER_SIZE as u64;
        self.current_sequence = self.start_sequence;

        while self.current_sequence < sequence {
            match self.read_entry() {
                Ok(Some(_)) => continue,
                Ok(None) => return Err(WalError::InvalidHeader(
                    format!("sequence {} not found", sequence)
                )),
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), WalError> {
        self.file
            .seek(SeekFrom::Start(HEADER_SIZE as u64))
            .map_err(|e| WalError::IOError(e))?;
        self.file_offset = HEADER_SIZE as u64;
        self.current_sequence = self.start_sequence;
        Ok(())
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

    pub fn iter(&mut self) -> WalIterator<'_> {
        WalIterator { reader: self }
    }
}

pub struct WalIterator<'a> {
    reader: &'a mut WalReader,
}

impl<'a> Iterator for WalIterator<'a> {
    type Item = Result<LogEntry, WalError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_entry() {
            Ok(Some(entry)) => Some(Ok(entry)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}
