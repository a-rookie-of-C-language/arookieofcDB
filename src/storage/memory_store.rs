use dashmap::DashMap;
use std::sync::Arc;

#[derive(Default)]
pub struct MemoryStore {
    data: DashMap<Vec<u8>, Vec<u8>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
        }
    }

    pub fn set(&self, key: &[u8], value: &[u8]) {
        self.data.insert(key.to_vec(), value.to_vec());
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(key).map(|v| v.clone())
    }

    pub fn delete(&self, key: &[u8]) -> bool {
        self.data.remove(key).is_some()
    }

    pub fn contains_key(&self, key: &[u8]) -> bool {
        self.data.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn iter(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.data.iter().map(|entry| (entry.key().clone(), entry.value().clone())).collect()
    }

    
}

pub type SharedMemoryStore = Arc<MemoryStore>;
