use crate::key_codec::KeyEncoding;

pub struct KeySerializer;

impl KeySerializer {
    pub fn serialize(key: &KeyEncoding, data: &mut [u8]) -> usize {
        match key {
            KeyEncoding::Int(v) => {
                data[0] = 0; // Type Int
                data[1..9].copy_from_slice(&v.to_be_bytes());
                9
            }
            KeyEncoding::Raw(s) => {
                data[0] = 1; // Type String (Raw)
                let bytes = s.as_bytes();
                let len = bytes.len() as u16;
                data[1..3].copy_from_slice(&len.to_be_bytes());
                data[3..3 + bytes.len()].copy_from_slice(bytes);
                3 + bytes.len()
            }
            KeyEncoding::Float(bits) => {
                data[0] = 2; // Type Float
                data[1..9].copy_from_slice(&bits.to_be_bytes());
                9
            }
        }
    }

    pub fn deserialize(data: &[u8]) -> (KeyEncoding, usize) {
        match data[0] {
            0 => {
                let v = i64::from_be_bytes(data[1..9].try_into().unwrap());
                (KeyEncoding::Int(v), 9)
            }
            1 => {
                let len = u16::from_be_bytes(data[1..3].try_into().unwrap()) as usize;
                let s = String::from_utf8(data[3..3 + len].to_vec()).expect("Invalid UTF-8 key");
                (KeyEncoding::Raw(s), 3 + len)
            }
            2 => {
                let bits = u64::from_be_bytes(data[1..9].try_into().unwrap());
                (KeyEncoding::Float(bits), 9)
            }
            _ => panic!("Unknown key type: {}", data[0]),
        }
    }
}
