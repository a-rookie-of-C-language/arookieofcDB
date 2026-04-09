use super::b_plus_tree_page_type::BPlusTreePageType;
use crate::storage::disk_manager::PageId;

pub struct BPlusTreePageHeader {
    pub page_type: BPlusTreePageType,
    pub parent_page_id: PageId,
    pub key_count: u16,
    pub next_page_id: PageId,
}

impl BPlusTreePageHeader {
    pub const SIZE: usize = 1 + 4 + 2 + 4; // 11 bytes

    pub fn serialize(&self, data: &mut [u8]) {
        data[0] = self.page_type as u8;
        data[1..5].copy_from_slice(&self.parent_page_id.to_be_bytes());
        data[5..7].copy_from_slice(&self.key_count.to_be_bytes());
        data[7..11].copy_from_slice(&self.next_page_id.to_be_bytes());
    }

    pub fn deserialize(data: &[u8]) -> Self {
        let page_type = BPlusTreePageType::from_u8(data[0]).expect("Invalid page type in header");
        let parent_page_id = u32::from_be_bytes(data[1..5].try_into().unwrap());
        let key_count = u16::from_be_bytes(data[5..7].try_into().unwrap());
        let next_page_id = u32::from_be_bytes(data[7..11].try_into().unwrap());

        Self {
            page_type,
            parent_page_id,
            key_count,
            next_page_id,
        }
    }
}
