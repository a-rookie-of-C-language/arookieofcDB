use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time;

use crate::wal::errors::WalError;
use crate::wal::log_entry::LogEntry;
use crate::wal::types::entry_type::EntryType;
use crate::wal::types::opration_type::OprationType;
use crate::wal::wal_config::WalConfig;
use crate::wal::wal_header::WalHeader;

const HEADER_SIZE: usize = 36;
const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024; // 64KB buffer

pub struct WalManager {
    config: WalConfig,
    current_file: File,
    current_file_number: u64,
    current_sequence: u64,
    file_offset: u64,
    start_sequence: u64,
    write_buffer: Vec<u8>,
    buffer_threshold: usize,
}

impl WalManager {
    pub fn new(config: WalConfig) -> Result<Self, WalError> {
        fs::create_dir_all(&config.wal_dir).map_err(|e| WalError::IOError(e))?;

        let mut files = Self::list_wal_files(&config.wal_dir)?;
        files.sort_by(|a, b| a.cmp(b));

        let (current_file_number, current_file, start_sequence, file_offset) =
            if files.is_empty() {
                let file_number = 1;
                let mut file = Self::create_new_file(&config.wal_dir, file_number)?;
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
                (file_number, file, 1, header_bytes.len() as u64)
            } else {
                let last_file_name = files.last().unwrap();
                let file_number = Self::parse_file_number(last_file_name)?;
                let file_path = config.wal_dir.join(last_file_name);
                let mut file = OpenOptions::new()
                    .create(false)
                    .write(true)
                    .read(true)
                    .open(&file_path)
                    .map_err(|e| WalError::IOError(e))?;

                let metadata = file.metadata().map_err(|e| WalError::IOError(e))?;
                let file_size = metadata.len();

                let mut header_bytes = [0u8; HEADER_SIZE];
                file.read_exact(&mut header_bytes)
                    .map_err(|e| WalError::IOError(e))?;
                let header = WalHeader::from_bytes(&header_bytes)?;

                file.seek(SeekFrom::End(0))
                    .map_err(|e| WalError::IOError(e))?;

                (file_number, file, header.get_start_sequence(), file_size)
            };

        let mut manager = Self {
            config,
            current_file,
            current_file_number,
            current_sequence: start_sequence,
            file_offset,
            start_sequence,
            write_buffer: Vec::with_capacity(DEFAULT_BUFFER_CAPACITY),
            buffer_threshold: DEFAULT_BUFFER_CAPACITY,
        };

        manager.load_last_sequence()?;

        Ok(manager)
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

    fn create_new_file(dir: &PathBuf, file_number: u64) -> Result<File, WalError> {
        let file_name = format!("{:06}.wal", file_number);
        let file_path = dir.join(file_name);

        OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(&file_path)
            .map_err(|e| WalError::IOError(e))
    }

    fn load_last_sequence(&mut self) -> Result<(), WalError> {
        let files = Self::list_wal_files(&self.config.wal_dir)?;
        if files.is_empty() {
            return Ok(());
        }

        let mut last_seq = self.start_sequence - 1;

        for file_name in files {
            let file_path = self.config.wal_dir.join(&file_name);
            let mut file = File::open(&file_path).map_err(|e| WalError::IOError(e))?;

            let mut header_bytes = [0u8; HEADER_SIZE];
            file.read_exact(&mut header_bytes)
                .map_err(|e| WalError::IOError(e))?;
            let _header = WalHeader::from_bytes(&header_bytes)?;

            let metadata = file.metadata().map_err(|e| WalError::IOError(e))?;
            let file_size = metadata.len();
            let mut offset = HEADER_SIZE as u64;

            while offset < file_size {
                if file_size - offset < 33 {
                    break;
                }

                let mut key_len_bytes = [0u8; 4];
                file.seek(SeekFrom::Start(offset + 10))
                    .map_err(|e| WalError::IOError(e))?;
                file.read_exact(&mut key_len_bytes)
                    .map_err(|e| WalError::IOError(e))?;
                let key_len = u32::from_be_bytes(key_len_bytes);

                file.seek(SeekFrom::Start(offset + 14 + key_len as u64))
                    .map_err(|e| WalError::IOError(e))?;
                let mut value_len_bytes = [0u8; 4];
                file.read_exact(&mut value_len_bytes)
                    .map_err(|e| WalError::IOError(e))?;
                let value_len = u32::from_be_bytes(value_len_bytes);

                let entry_size = LogEntry::entry_size(key_len as usize, value_len as usize);
                let entry_bytes = (offset + entry_size as u64).min(file_size);

                file.seek(SeekFrom::Start(offset))
                    .map_err(|e| WalError::IOError(e))?;
                let mut buffer = vec![0u8; (entry_bytes - offset) as usize];
                file.read_exact(&mut buffer)
                    .map_err(|e| WalError::IOError(e))?;

                if let Ok(entry) = LogEntry::from_bytes(&buffer) {
                    if entry.seq > last_seq {
                        last_seq = entry.seq;
                    }
                }

                offset = entry_bytes;
            }
        }

        self.current_sequence = last_seq + 1;
        Ok(())
    }

    pub fn roll_file(&mut self) -> Result<(), WalError> {
        self.flush_buffer()?;

        self.current_file_number += 1;
        let mut new_file = Self::create_new_file(&self.config.wal_dir, self.current_file_number)?;

        let header = WalHeader::new(
            "arookieofcDB".to_string(),
            1,
            self.current_sequence,
            time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );

        let header_bytes = header.to_bytes();
        new_file.write_all(&header_bytes)
            .map_err(|e| WalError::IOError(e))?;
        new_file.sync_all().map_err(|e| WalError::IOError(e))?;

        self.current_file = new_file;
        self.file_offset = header_bytes.len() as u64;

        Ok(())
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
        self.append_to_buffer(&entry_bytes)?;

        self.current_sequence += 1;

        Ok(seq)
    }

    pub fn write_raw_entry(&mut self, entry_bytes: &[u8]) -> Result<(), WalError> {
        self.append_to_buffer(entry_bytes)?;
        self.current_sequence += 1;
        Ok(())
    }

    fn append_to_buffer(&mut self, entry_bytes: &[u8]) -> Result<(), WalError> {
        if self.write_buffer.len() + entry_bytes.len() > self.buffer_threshold {
            self.flush_buffer()?;
        }

        if self.file_offset + self.write_buffer.len() as u64 + entry_bytes.len() as u64 > self.config.max_file_size {
            self.flush_buffer()?;
            self.roll_file()?;
        }

        self.write_buffer.extend_from_slice(entry_bytes);
        self.file_offset += entry_bytes.len() as u64;

        Ok(())
    }

    pub fn flush_buffer(&mut self) -> Result<(), WalError> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }

        self.current_file
            .write_all(&self.write_buffer)
            .map_err(|e| WalError::IOError(e))?;
        self.current_file
            .sync_all()
            .map_err(|e| WalError::IOError(e))?;

        self.write_buffer.clear();

        Ok(())
    }

    pub fn write_checkpoint(&mut self) -> Result<u64, WalError> {
        let seq = self.current_sequence;
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
        self.append_to_buffer(&entry_bytes)?;

        Ok(seq)
    }

    pub fn cleanup_old_files(&mut self, checkpoint_sequence: u64) -> Result<(), WalError> {
        let files = Self::list_wal_files(&self.config.wal_dir)?;

        for file_name in files {
            let file_number = match Self::parse_file_number(&file_name) {
                Ok(num) => num,
                Err(_) => continue,
            };

            if file_number + (self.config.max_retained_files as u64) < self.current_file_number {
                let file_path = self.config.wal_dir.join(&file_name);
                fs::remove_file(&file_path).map_err(|e| WalError::IOError(e))?;
            }
        }

        Ok(())
    }

    pub fn current_sequence(&self) -> u64 {
        self.current_sequence
    }

    pub fn file_offset(&self) -> u64 {
        self.file_offset
    }

    pub fn set_buffer_threshold(&mut self, threshold: usize) {
        self.buffer_threshold = threshold;
    }
}

impl Drop for WalManager {
    fn drop(&mut self) {
        let _ = self.flush_buffer();
    }
}
