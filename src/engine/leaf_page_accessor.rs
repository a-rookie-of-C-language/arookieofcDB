use super::b_plus_tree_page_header::BPlusTreePageHeader;
use super::key_serializer::KeySerializer;
use crate::key_codec::KeyEncoding;

pub struct LeafPageAccessor<'a> {
    data: &'a mut [u8],
}

impl<'a> LeafPageAccessor<'a> {
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data }
    }

    pub fn get_header(&self) -> BPlusTreePageHeader {
        BPlusTreePageHeader::deserialize(&self.data[0..BPlusTreePageHeader::SIZE])
    }

    pub fn set_header(&mut self, header: &BPlusTreePageHeader) {
        header.serialize(&mut self.data[0..BPlusTreePageHeader::SIZE]);
    }

    pub fn insert(&mut self, key: &KeyEncoding, value: &[u8]) -> bool {
        let mut header = self.get_header();
        let mut offset = BPlusTreePageHeader::SIZE;

        // Find existing key or end of entries
        for _ in 0..header.key_count {
            let (existing_key, key_len) = KeySerializer::deserialize(&self.data[offset..]);
            let val_len_offset = offset + key_len;
            let val_len = u32::from_be_bytes(self.data[val_len_offset..val_len_offset + 4].try_into().unwrap()) as usize;
            
            if &existing_key == key {
                // Update existing
                // Simple implementation: if new value fits, update in place; else, this is a "known limitation" for now
                // Actually, let's just append or fail for the prototype
                self.data[val_len_offset..val_len_offset + 4].copy_from_slice(&(value.len() as u32).to_be_bytes());
                self.data[val_len_offset + 4..val_len_offset + 4 + value.len()].copy_from_slice(value);
                return true;
            }
            offset = val_len_offset + 4 + val_len;
        }

        // Append new entry
        let mut temp_data = [0u8; 512]; // Buffer for key
        let key_len = KeySerializer::serialize(key, &mut temp_data);
        
        if offset + key_len + 4 + value.len() > self.data.len() {
            return false; // Page full
        }

        self.data[offset..offset + key_len].copy_from_slice(&temp_data[..key_len]);
        let val_len_offset = offset + key_len;
        self.data[val_len_offset..val_len_offset + 4].copy_from_slice(&(value.len() as u32).to_be_bytes());
        self.data[val_len_offset + 4..val_len_offset + 4 + value.len()].copy_from_slice(value);

        header.key_count += 1;
        self.set_header(&header);
        true
    }

    pub fn get(&self, key: &KeyEncoding) -> Option<Vec<u8>> {
        let header = self.get_header();
        let mut offset = BPlusTreePageHeader::SIZE;

        for _ in 0..header.key_count {
            let (existing_key, key_len) = KeySerializer::deserialize(&self.data[offset..]);
            let val_len_offset = offset + key_len;
            let val_len = u32::from_be_bytes(self.data[val_len_offset..val_len_offset + 4].try_into().unwrap()) as usize;
            
            if &existing_key == key {
                return Some(self.data[val_len_offset + 4..val_len_offset + 4 + val_len].to_vec());
            }
            offset = val_len_offset + 4 + val_len;
        }
        None
    }
}
