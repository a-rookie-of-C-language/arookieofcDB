use macros::all_args_constructor;

#[all_args_constructor]
pub struct WalHeader {
    magic: String,
    version: u32,
    start_sequence: u64,
    create_time: u64,
}

impl WalHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend(self.magic.as_bytes());
        header.extend(self.version.to_be_bytes());
        header.extend(self.start_sequence.to_be_bytes());
        header.extend(self.create_time.to_be_bytes());
        header
    }
}
