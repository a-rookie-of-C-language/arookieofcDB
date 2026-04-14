use crate::wal::errors::WalError;
use macros::{all_args_constructor, getter};

const HEADER_SIZE: usize = 48;
const MAGIC_SIZE: usize = 16;

#[getter]
#[all_args_constructor]
pub struct WalHeader {
    magic: String,
    version: u32,
    start_sequence: u64,
    create_time: u64,
}

impl WalHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut header = Vec::with_capacity(HEADER_SIZE);
        
        let mut magic_bytes = self.magic.as_bytes().to_vec();
        if magic_bytes.len() < MAGIC_SIZE {
            magic_bytes.resize(MAGIC_SIZE, 0);
        } else if magic_bytes.len() > MAGIC_SIZE {
            magic_bytes.truncate(MAGIC_SIZE);
        }
        header.extend(magic_bytes);
        
        header.extend(self.version.to_be_bytes());
        header.extend(self.start_sequence.to_be_bytes());
        header.extend(self.create_time.to_be_bytes());
        
        header
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, WalError> {
        if bytes.len() < HEADER_SIZE {
            return Err(WalError::InvalidHeader("header too short".to_string()));
        }

        let magic = String::from_utf8_lossy(&bytes[0..MAGIC_SIZE])
            .trim_end_matches('\0')
            .to_string();

        if magic != "arookieofcDB" {
            return Err(WalError::InvalidHeader(format!(
                "invalid magic: {}",
                magic
            )));
        }

        let version = u32::from_be_bytes(bytes[MAGIC_SIZE..MAGIC_SIZE + 4].try_into().unwrap());
        let start_sequence = u64::from_be_bytes(
            bytes[MAGIC_SIZE + 4..MAGIC_SIZE + 12].try_into().unwrap(),
        );
        let create_time = u64::from_be_bytes(
            bytes[MAGIC_SIZE + 12..MAGIC_SIZE + 20].try_into().unwrap(),
        );

        Ok(Self {
            magic,
            version,
            start_sequence,
            create_time,
        })
    }
}
