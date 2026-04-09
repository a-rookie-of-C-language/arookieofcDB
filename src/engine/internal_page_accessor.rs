use super::b_plus_tree_page_header::BPlusTreePageHeader;
use super::key_serializer::KeySerializer;
use crate::key_codec::KeyEncoding;
use crate::storage::disk_manager::PageId;

pub struct InternalPageAccessor<'a> {
    data: &'a mut [u8],
}

impl<'a> InternalPageAccessor<'a> {
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data }
    }

    pub fn get_header(&self) -> BPlusTreePageHeader {
        BPlusTreePageHeader::deserialize(&self.data[0..BPlusTreePageHeader::SIZE])
    }

    pub fn set_header(&mut self, header: &BPlusTreePageHeader) {
        header.serialize(&mut self.data[0..BPlusTreePageHeader::SIZE]);
    }

    pub fn get_child_id(&self, key: &KeyEncoding) -> PageId {
        let header = self.get_header();
        let mut offset = BPlusTreePageHeader::SIZE;

        // Binary search or linear search for the correct child
        // Layout: [PageId 0] [Key 1] [PageId 1] [Key 2] [PageId 2] ...
        let first_child_id = u32::from_be_bytes(self.data[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let mut last_child_id = first_child_id;

        for _ in 0..header.key_count {
            let (existing_key, key_len) = KeySerializer::deserialize(&self.data[offset..]);
            let next_child_offset = offset + key_len;
            let next_child_id = u32::from_be_bytes(self.data[next_child_offset..next_child_offset + 4].try_into().unwrap());

            if key < &existing_key {
                return last_child_id;
            }

            last_child_id = next_child_id;
            offset = next_child_offset + 4;
        }

        last_child_id
    }

    pub fn insert(&mut self, key: &KeyEncoding, child_page_id: PageId) -> bool {
        let mut header = self.get_header();
        // Simplified: always append for now
        // Real implementation would find correct index
        
        // Skip header and find end
        let mut offset = BPlusTreePageHeader::SIZE;
        offset += 4; // Skip first child ID

        for _ in 0..header.key_count {
            let (_, key_len) = KeySerializer::deserialize(&self.data[offset..]);
            offset += key_len + 4;
        }

        let mut temp_data = [0u8; 512];
        let key_len = KeySerializer::serialize(key, &mut temp_data);

        if offset + key_len + 4 > self.data.len() {
            return false;
        }

        self.data[offset..offset + key_len].copy_from_slice(&temp_data[..key_len]);
        self.data[offset + key_len..offset + key_len + 4].copy_from_slice(&child_page_id.to_be_bytes());

        header.key_count += 1;
        self.set_header(&header);
        true
    }
}
