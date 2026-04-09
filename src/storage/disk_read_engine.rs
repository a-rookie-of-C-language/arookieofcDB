use crate::key_codec::KeyEncoding;
use super::kv_engine::KvEngine;

pub trait DiskReadEngine: KvEngine {
    fn get_disk_only(&mut self, key: &KeyEncoding) -> Option<Vec<u8>> {
        self.get(key).map(|v| v.to_vec())
    }
}