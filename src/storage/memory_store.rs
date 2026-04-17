use std::collections::HashMap;
use std::sync::{RwLock, Arc};

#[derive(Default)]
pub struct MemoryStore {
    data: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    pub fn set(&self, key: &[u8], value: &[u8]) {
        let mut data = self.data.write().unwrap();
        data.insert(key.to_vec(), value.to_vec());
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let data = self.data.read().unwrap();
        data.get(key).cloned()
    }

    pub fn delete(&self, key: &[u8]) -> bool {
        let mut data = self.data.write().unwrap();
        data.remove(key).is_some()
    }

    pub fn contains_key(&self, key: &[u8]) -> bool {
        let data = self.data.read().unwrap();
        data.contains_key(key)
    }

    pub fn len(&self) -> usize {
        let data = self.data.read().unwrap();
        data.len()
    }

    pub fn is_empty(&self) -> bool {
        let data = self.data.read().unwrap();
        data.is_empty()
    }

    pub fn iter(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        let data = self.data.read().unwrap();
        data.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

pub type SharedMemoryStore = Arc<MemoryStore>;
