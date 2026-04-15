use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::wal::errors::WalError;
use crate::wal::log_entry::LogEntry;
use crate::wal::wal_config::WalConfig;
use crate::wal::wal_header::WalHeader;

const HEADER_SIZE: usize = 48;

pub struct WalReader {
    config: WalConfig,
    files: Vec<String>,
    current_file_index: usize,
    current_file: Option<File>,
    current_sequence: u64,
    file_offset: u64,
    start_sequence: u64,
    file_size: u64,
}

impl WalReader {
    pub fn new(config: WalConfig) -> Result<Self, WalError> {
        fs::create_dir_all(&config.wal_dir).map_err(|e| WalError::IOError(e))?;

        let mut files = Self::list_wal_files(&config.wal_dir)?;
        files.sort_by(|a, b| a.cmp(b));

        if files.is_empty() {
            return Err(WalError::FileNotFound);
        }

        let (current_file, start_sequence, file_size) =
            Self::open_file(&config.wal_dir, &files[0])?;

        Ok(Self {
            config,
            files,
            current_file_index: 0,
            current_file: Some(current_file),
            current_sequence: start_sequence,
            file_offset: HEADER_SIZE as u64,
            start_sequence,
            file_size,
        })
    }

    pub fn new_from_path(path: &str) -> Result<Self, WalError> {
        let config = WalConfig::new(path);
        Self::new(config)
    }

    fn list_wal_files(dir: &PathBuf) -> Result<Vec<String>, WalError> {
        let mut files = Vec::new();
        let entries = fs::read_dir(dir).map_err(|e| WalError::IOError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| WalError::IOError(e))?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.ends_with(".wal") {
                files.push(file_name);
            }
        }

        Ok(files)
    }

    fn parse_file_number(file_name: &str) -> Result<u64, WalError> {
        let num_str = file_name.trim_end_matches(".wal");
        num_str.parse::<u64>().map_err(|_| {
            WalError::InvalidHeader(format!("invalid wal file name: {}", file_name))
        })
    }

    fn open_file(dir: &PathBuf, file_name: &str) -> Result<(File, u64, u64), WalError> {
        let file_path = dir.join(file_name);
        let mut file = OpenOptions::new()
            .read(true)
            .open(&file_path)
            .map_err(|e| WalError::IOError(e))?;

        let metadata = file.metadata().map_err(|e| WalError::IOError(e))?;
        let file_size = metadata.len();

        if file_size < HEADER_SIZE as u64 {
            return Err(WalError::InvalidHeader("file too short".to_string()));
        }

        let mut header_bytes = [0u8; HEADER_SIZE];
        file.read_exact(&mut header_bytes)
            .map_err(|e| WalError::IOError(e))?;
        let header = WalHeader::from_bytes(&header_bytes)?;

        file.seek(SeekFrom::Start(HEADER_SIZE as u64))
            .map_err(|e| WalError::IOError(e))?;

        Ok((file, header.get_start_sequence(), file_size))
    }

    fn advance_to_next_file(&mut self) -> Result<bool, WalError> {
        if self.current_file_index >= self.files.len() - 1 {
            return Ok(false);
        }

        self.current_file_index += 1;
        let file_name = &self.files[self.current_file_index];

        let (file, start_sequence, file_size) =
            Self::open_file(&self.config.wal_dir, file_name)?;

        self.current_file = Some(file);
        self.current_sequence = start_sequence;
        self.file_offset = HEADER_SIZE as u64;
        self.file_size = file_size;

        Ok(true)
    }

    pub fn read_entry(&mut self) -> Result<Option<LogEntry>, WalError> {
        loop {
            if self.file_offset >= self.file_size {
                if !self.advance_to_next_file()? {
                    return Ok(None);
                }
                continue;
            }

            let remaining = (self.file_size - self.file_offset) as usize;
            if remaining < 33 {
                if !self.advance_to_next_file()? {
                    return Ok(None);
                }
                continue;
            }

            let file = self
                .current_file
                .as_mut()
                .ok_or(WalError::InvalidHeader("no current file".to_string()))?;

            let mut key_len_bytes = [0u8; 4];
            file.seek(SeekFrom::Start(self.file_offset + 10))
                .map_err(|e| WalError::IOError(e))?;
            file.read_exact(&mut key_len_bytes)
                .map_err(|e| WalError::IOError(e))?;
            let key_len = u32::from_be_bytes(key_len_bytes);

            file.seek(SeekFrom::Start(self.file_offset + 14 + key_len as u64))
                .map_err(|e| WalError::IOError(e))?;
            let mut value_len_bytes = [0u8; 4];
            file.read_exact(&mut value_len_bytes)
                .map_err(|e| WalError::IOError(e))?;
            let value_len = u32::from_be_bytes(value_len_bytes);

            let entry_size = LogEntry::entry_size(key_len as usize, value_len as usize);
            let entry_bytes = (self.file_offset + entry_size as u64).min(self.file_size);
            let actual_size = (entry_bytes - self.file_offset) as usize;

            if actual_size < entry_size {
                if !self.advance_to_next_file()? {
                    return Ok(None);
                }
                continue;
            }

            file.seek(SeekFrom::Start(self.file_offset))
                .map_err(|e| WalError::IOError(e))?;

            let mut buffer = vec![0u8; entry_size];
            file.read_exact(&mut buffer)
                .map_err(|e| WalError::IOError(e))?;

            let entry = LogEntry::from_bytes(&buffer)?;

            if !entry.verify_checksum() {
                return Err(WalError::ChecksumMismatch);
            }

            self.current_sequence = entry.seq;
            self.file_offset += entry_size as u64;

            return Ok(Some(entry));
        }
    }

    pub fn seek_to(&mut self, sequence: u64) -> Result<(), WalError> {
        if sequence < self.start_sequence {
            return Err(WalError::InvalidHeader(format!(
                "sequence {} before start sequence {}",
                sequence, self.start_sequence
            )));
        }

        self.current_file_index = 0;
        let (file, start_sequence, file_size) =
            Self::open_file(&self.config.wal_dir, &self.files[0])?;

        self.current_file = Some(file);
        self.current_sequence = start_sequence;
        self.file_offset = HEADER_SIZE as u64;
        self.file_size = file_size;

        loop {
            if self.current_sequence >= sequence {
                break;
            }

            match self.read_entry() {
                Ok(Some(_)) => continue,
                Ok(None) => {
                    return Err(WalError::InvalidHeader(format!(
                        "sequence {} not found",
                        sequence
                    )));
                }
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), WalError> {
        if self.files.is_empty() {
            return Err(WalError::FileNotFound);
        }

        self.current_file_index = 0;
        let (file, start_sequence, file_size) =
            Self::open_file(&self.config.wal_dir, &self.files[0])?;

        self.current_file = Some(file);
        self.current_sequence = start_sequence;
        self.file_offset = HEADER_SIZE as u64;
        self.file_size = file_size;

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

    pub fn current_file_name(&self) -> Option<&str> {
        self.files.get(self.current_file_index).map(|s| s.as_str())
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
