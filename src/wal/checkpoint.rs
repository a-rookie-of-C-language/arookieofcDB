use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time;

use crate::wal::errors::WalError;
use crate::wal::wal_config::WalConfig;

const CHECKPOINT_HEADER_SIZE: usize = 40;
const CHECKPOINT_MAGIC: &[u8; 8] = b"AROOKIED";

#[derive(Debug)]
pub struct Checkpoint {
    version: u32,
    sequence: u64,
    create_time: u64,
    entry_count: u64,
    data: HashMap<Vec<u8>, Vec<u8>>,
}

impl Checkpoint {
    pub fn new(sequence: u64, data: HashMap<Vec<u8>, Vec<u8>>) -> Self {
        Self {
            version: 1,
            sequence,
            create_time: time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            entry_count: data.len() as u64,
            data,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        bytes.extend_from_slice(CHECKPOINT_MAGIC);
        bytes.extend(self.version.to_be_bytes());
        bytes.extend(self.sequence.to_be_bytes());
        bytes.extend(self.create_time.to_be_bytes());
        bytes.extend(self.entry_count.to_be_bytes());

        for (key, value) in &self.data {
            bytes.extend(key.len().to_be_bytes());
            bytes.extend(key);
            bytes.extend(value.len().to_be_bytes());
            bytes.extend(value);
        }

        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WalError> {
        if bytes.len() < CHECKPOINT_HEADER_SIZE {
            return Err(WalError::InvalidHeader("checkpoint too short".to_string()));
        }

        let mut offset = 0;
        let mut magic = [0u8; 8];
        magic.copy_from_slice(&bytes[offset..offset + 8]);
        if magic != *CHECKPOINT_MAGIC {
            return Err(WalError::InvalidHeader("invalid checkpoint magic".to_string()));
        }
        offset += 8;

        let version = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let sequence = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let create_time = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let entry_count = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let mut data = HashMap::new();
        for _ in 0..entry_count {
            if bytes.len() - offset < 8 {
                return Err(WalError::InvalidHeader("truncated checkpoint entry".to_string()));
            }

            let key_len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
            offset += 4;

            if bytes.len() < offset + key_len as usize {
                return Err(WalError::InvalidHeader("truncated key".to_string()));
            }
            let key = bytes[offset..offset + key_len as usize].to_vec();
            offset += key_len as usize;

            if bytes.len() - offset < 4 {
                return Err(WalError::InvalidHeader("truncated value len".to_string()));
            }
            let value_len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
            offset += 4;

            if bytes.len() < offset + value_len as usize {
                return Err(WalError::InvalidHeader("truncated value".to_string()));
            }
            let value = bytes[offset..offset + value_len as usize].to_vec();
            offset += value_len as usize;

            data.insert(key, value);
        }

        Ok(Self {
            version,
            sequence,
            create_time,
            entry_count,
            data,
        })
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }

    pub fn data(&self) -> &HashMap<Vec<u8>, Vec<u8>> {
        &self.data
    }

    pub fn create_time(&self) -> u64 {
        self.create_time
    }
}

pub struct CheckpointManager {
    config: WalConfig,
}

impl CheckpointManager {
    pub fn new(config: WalConfig) -> Self {
        fs::create_dir_all(&config.wal_dir).unwrap();
        Self { config }
    }

    pub fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<String, WalError> {
        let file_name = format!("checkpoint_{:016}.cp", checkpoint.sequence());
        let file_path = self.config.wal_dir.join(&file_name);

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&file_path)
            .map_err(|e| WalError::IOError(e))?;

        let bytes = checkpoint.to_bytes();
        file.write_all(&bytes)
            .map_err(|e| WalError::IOError(e))?;
        file.sync_all().map_err(|e| WalError::IOError(e))?;

        self.update_latest_checkpoint_link(&file_name)?;

        Ok(file_name)
    }

    fn update_latest_checkpoint_link(&self, file_name: &str) -> Result<(), WalError> {
        let link_path = self.config.wal_dir.join("checkpoint_latest.cp");
        
        if link_path.exists() {
            fs::remove_file(&link_path).map_err(|e| WalError::IOError(e))?;
        }

        fs::hard_link(
            self.config.wal_dir.join(file_name),
            &link_path,
        ).or_else(|_| {
            fs::copy(
                self.config.wal_dir.join(file_name),
                &link_path,
            ).map(|_| ()).map_err(|e| WalError::IOError(e))
        })
    }

    pub fn load_latest_checkpoint(&self) -> Result<Option<Checkpoint>, WalError> {
        let link_path = self.config.wal_dir.join("checkpoint_latest.cp");
        
        if !link_path.exists() {
            return Ok(None);
        }

        let mut file = File::open(&link_path).map_err(|e| WalError::IOError(e))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| WalError::IOError(e))?;

        let checkpoint = Checkpoint::from_bytes(&bytes)?;
        Ok(Some(checkpoint))
    }

    pub fn list_checkpoints(&self) -> Result<Vec<String>, WalError> {
        let mut checkpoints = Vec::new();
        let entries = fs::read_dir(&self.config.wal_dir).map_err(|e| WalError::IOError(e))?;

        for entry in entries {
            let entry = entry.map_err(|e| WalError::IOError(e))?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.starts_with("checkpoint_") && file_name.ends_with(".cp") {
                checkpoints.push(file_name);
            }
        }

        checkpoints.sort_by(|a, b| b.cmp(a));
        Ok(checkpoints)
    }

    pub fn cleanup_old_checkpoints(&self, max_retain: usize) -> Result<usize, WalError> {
        let mut checkpoints = self.list_checkpoints()?;
        checkpoints.retain(|c| c != "checkpoint_latest.cp");

        if checkpoints.len() <= max_retain {
            return Ok(0);
        }

        let mut removed_count = 0;
        for file_name in checkpoints.iter().skip(max_retain) {
            let file_path = self.config.wal_dir.join(file_name);
            fs::remove_file(&file_path).map_err(|e| WalError::IOError(e))?;
            removed_count += 1;
        }

        Ok(removed_count)
    }

    pub fn get_checkpoint_path(&self, sequence: u64) -> PathBuf {
        self.config.wal_dir.join(format!("checkpoint_{:016}.cp", sequence))
    }
}
