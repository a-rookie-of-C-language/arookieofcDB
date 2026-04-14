use crate::wal::types::entry_type::EntryType;
use crate::wal::types::opration_type::OprationType;
use macros::all_args_constructor;

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
}