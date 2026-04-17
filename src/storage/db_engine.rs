use std::sync::{Arc, Mutex};

use crate::storage::memory_store::MemoryStore;
use crate::wal::checkpoint::{Checkpoint, CheckpointManager};
use crate::wal::errors::WalError;
use crate::wal::log_entry::LogEntry;
use crate::wal::types::entry_type::EntryType;
use crate::wal::types::opration_type::OprationType;
use crate::wal::wal_config::WalConfig;
use crate::wal::wal_manager::WalManager;
use crate::wal::wal_reader::WalReader;

pub struct DbEngine {
    memory_store: Arc<MemoryStore>,
    wal_manager: Mutex<WalManager>,
    checkpoint_manager: CheckpointManager,
    config: WalConfig,
}

impl DbEngine {
    pub fn new(config: WalConfig) -> Result<Self, WalError> {
        let memory_store = Arc::new(MemoryStore::new());
        let wal_manager = Mutex::new(WalManager::new(config.clone())?);
        let checkpoint_manager = CheckpointManager::new(config.clone());

        let mut engine = Self {
            memory_store,
            wal_manager,
            checkpoint_manager,
            config,
        };

        engine.recover_from_wal()?;

        Ok(engine)
    }

    pub fn set(&self, key: &[u8], value: &[u8]) -> Result<(), WalError> {
        let mut wal_manager = self.wal_manager.lock().unwrap();
        wal_manager.write_entry(key, value)?;
        self.memory_store.set(key, value);
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.memory_store.get(key)
    }

    pub fn delete(&self, key: &[u8]) -> Result<bool, WalError> {
        let mut wal_manager = self.wal_manager.lock().unwrap();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let seq = wal_manager.current_sequence();
        let data = LogEntry::build_entry_data(seq, key, b"", timestamp);
        let checksum = LogEntry::crc32(&data);
        
        let entry = LogEntry::new(
            seq,
            EntryType::Log,
            OprationType::Delete,
            key.to_vec(),
            Vec::new(),
            checksum,
            timestamp,
        );
        
        let entry_bytes = entry.to_bytes();
        
        if wal_manager.file_offset() + entry_bytes.len() as u64 > self.config.max_file_size {
            wal_manager.roll_file()?;
        }
        
        wal_manager.write_raw_entry(&entry_bytes)?;
        
        let result = self.memory_store.delete(key);
        Ok(result)
    }

    pub fn contains_key(&self, key: &[u8]) -> bool {
        self.memory_store.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.memory_store.len()
    }

    pub fn create_checkpoint(&self) -> Result<u64, WalError> {
        let data: std::collections::HashMap<Vec<u8>, Vec<u8>> = self.memory_store.iter().into_iter().collect();
        let sequence = self.wal_manager.lock().unwrap().current_sequence() - 1;
        
        let checkpoint = Checkpoint::new(sequence, data);
        self.checkpoint_manager.save_checkpoint(&checkpoint)?;
        
        let mut wal_manager = self.wal_manager.lock().unwrap();
        wal_manager.cleanup_old_files(sequence)?;
        
        Ok(sequence)
    }

    fn recover_from_wal(&mut self) -> Result<(), WalError> {
        if let Ok(mut reader) = WalReader::new(self.config.clone()) {
            while let Ok(Some(entry)) = reader.read_entry() {
                match entry.opration_type {
                    OprationType::Insert => {
                        self.memory_store.set(&entry.key, &entry.value);
                    }
                    OprationType::Delete => {
                        self.memory_store.delete(&entry.key);
                    }
                    OprationType::Update => {
                        self.memory_store.set(&entry.key, &entry.value);
                    }
                }
            }
        }
        
        if let Ok(Some(checkpoint)) = self.checkpoint_manager.load_latest_checkpoint() {
            for (key, value) in checkpoint.data() {
                self.memory_store.set(key, value);
            }
        }
        
        Ok(())
    }

    pub fn memory_store(&self) -> Arc<MemoryStore> {
        Arc::clone(&self.memory_store)
    }
}

pub type SharedDbEngine = Arc<DbEngine>;
