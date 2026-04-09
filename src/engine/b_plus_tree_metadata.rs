use crate::storage::disk_manager::PageId;

pub struct BPlusTreeMetadata {
    pub root_page_id: PageId,
}

impl BPlusTreeMetadata {
    pub fn serialize(&self, data: &mut [u8]) {
        data[0..4].copy_from_slice(&self.root_page_id.to_be_bytes());
    }

    pub fn deserialize(data: &[u8]) -> Self {
        let root_page_id = u32::from_be_bytes(data[0..4].try_into().unwrap());
        Self { root_page_id }
    }
}
