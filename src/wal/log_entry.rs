use crate::wal::types::entry_type::EntryType;
use crate::wal::types::opration_type::OprationType;
use macros::all_args_constructor;
use crate::wal::WalError;
#[all_args_constructor]
pub struct LogEntry {
    pub seq: u64,
    pub entry_type: EntryType,
    pub opration_type: OprationType,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub checksum: u32,
    pub timestamp: u64,
}

impl LogEntry {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.seq.to_be_bytes());
        bytes.extend(u8::from(self.entry_type).to_be_bytes());
        bytes.extend(u8::from(self.opration_type).to_be_bytes());
        bytes.extend(self.key.len().to_be_bytes());
        bytes.extend(&self.key);
        bytes.extend(self.value.len().to_be_bytes());
        bytes.extend(&self.value);
        bytes.extend(self.checksum.to_be_bytes());
        bytes.extend(self.timestamp.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WalError> {
        if bytes.len() < 33 {
            return Err(WalError::UnexpectedEof);
        }

        let mut offset = 0;
        let seq = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;

        let entry_type = match bytes[offset] {
            0 => EntryType::Log,
            1 => EntryType::Checkpoint,
            _ => return Err(WalError::Corrupted("invalid entry type".to_string())),
        };
        offset += 1;

        let opration_type = match bytes[offset] {
            0 => OprationType::Insert,
            1 => OprationType::Delete,
            2 => OprationType::Update,
            _ => return Err(WalError::Corrupted("invalid operation type".to_string())),
        };
        offset += 1;

        let key_len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let key = bytes[offset..offset + key_len as usize].to_vec();
        offset += key_len as usize;

        let value_len = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let value = bytes[offset..offset + value_len as usize].to_vec();
        offset += value_len as usize;

        let checksum = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;

        let timestamp = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());

        Ok(Self {
            seq,
            entry_type,
            opration_type,
            key,
            value,
            checksum,
            timestamp,
        })
    }

    pub fn entry_size(key_len: usize, value_len: usize) -> usize {
        8 + 1 + 1 + 4 + key_len + 4 + value_len + 4 + 8
    }

    pub fn verify_checksum(&self) -> bool {
        let data = Self::build_entry_data(self.seq, &self.key, &self.value, self.timestamp);
        let computed = Self::crc32(&data);
        computed == self.checksum
    }

    fn build_entry_data(seq: u64, key: &[u8], value: &[u8], timestamp: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(seq.to_be_bytes());
        data.extend(key.len().to_be_bytes());
        data.extend(key);
        data.extend(value.len().to_be_bytes());
        data.extend(value);
        data.extend(timestamp.to_be_bytes());
        data
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFFFFFFu32;
        for &byte in data {
            crc ^= byte as u32;
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB88320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }
}