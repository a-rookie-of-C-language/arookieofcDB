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