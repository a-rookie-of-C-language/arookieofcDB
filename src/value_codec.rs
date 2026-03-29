#[derive(Debug, Clone, PartialEq)]
pub enum StringEncoding {
    Raw(Vec<u8>),
    Int(i64),
    Float(f64),
}

const MAGIC: &[u8; 3] = b"SE1";
const TAG_RAW: u8 = 0;
const TAG_INT: u8 = 1;
const TAG_FLOAT: u8 = 2;

impl StringEncoding {
    pub fn from_input(input: &str) -> Self {
        if let Ok(v) = input.parse::<i64>() {
            return Self::Int(v);
        }

        // Float detection: only when it looks like a float literal.
        let looks_float = input.contains('.') || input.contains('e') || input.contains('E');
        if looks_float {
            if let Ok(v) = input.parse::<f64>() {
                if v.is_finite() {
                    return Self::Float(v);
                }
            }
        }

        Self::Raw(input.as_bytes().to_vec())
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);

        match self {
            Self::Raw(bytes) => {
                out.push(TAG_RAW);
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            }
            Self::Int(v) => {
                out.push(TAG_INT);
                out.extend_from_slice(&v.to_le_bytes());
            }
            Self::Float(v) => {
                out.push(TAG_FLOAT);
                out.extend_from_slice(&v.to_bits().to_le_bytes());
            }
        }

        out
    }

    pub fn decode(bytes: &[u8]) -> Self {
        if bytes.len() < 4 || &bytes[..3] != MAGIC {
            return Self::Raw(bytes.to_vec());
        }

        match bytes[3] {
            TAG_RAW => {
                if bytes.len() < 8 {
                    return Self::Raw(bytes.to_vec());
                }
                let mut len_bytes = [0u8; 4];
                len_bytes.copy_from_slice(&bytes[4..8]);
                let len = u32::from_le_bytes(len_bytes) as usize;
                if bytes.len() != 8 + len {
                    return Self::Raw(bytes.to_vec());
                }
                Self::Raw(bytes[8..].to_vec())
            }
            TAG_INT => {
                if bytes.len() != 12 {
                    return Self::Raw(bytes.to_vec());
                }
                let mut int_bytes = [0u8; 8];
                int_bytes.copy_from_slice(&bytes[4..12]);
                Self::Int(i64::from_le_bytes(int_bytes))
            }
            TAG_FLOAT => {
                if bytes.len() != 12 {
                    return Self::Raw(bytes.to_vec());
                }
                let mut float_bytes = [0u8; 8];
                float_bytes.copy_from_slice(&bytes[4..12]);
                Self::Float(f64::from_bits(u64::from_le_bytes(float_bytes)))
            }
            _ => Self::Raw(bytes.to_vec()),
        }
    }

    pub fn to_display_string(&self) -> String {
        match self {
            Self::Raw(bytes) => String::from_utf8_lossy(bytes).to_string(),
            Self::Int(v) => v.to_string(),
            Self::Float(v) => v.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StringEncoding;

    #[test]
    fn detect_int() {
        assert!(matches!(
            StringEncoding::from_input("100"),
            StringEncoding::Int(100)
        ));
    }

    #[test]
    fn detect_float() {
        match StringEncoding::from_input("3.14") {
            StringEncoding::Float(v) => assert_eq!(v, 3.14),
            other => panic!("unexpected encoding: {other:?}"),
        }
    }

    #[test]
    fn raw_fallback() {
        assert!(matches!(
            StringEncoding::from_input("hello"),
            StringEncoding::Raw(_)
        ));
    }

    #[test]
    fn round_trip() {
        let items = [
            StringEncoding::Raw(b"abc".to_vec()),
            StringEncoding::Int(42),
            StringEncoding::Float(1.5),
        ];

        for item in items {
            let encoded = item.encode();
            let decoded = StringEncoding::decode(&encoded);
            assert_eq!(decoded, item);
        }
    }

    #[test]
    fn decode_legacy_raw_bytes() {
        let decoded = StringEncoding::decode(b"legacy");
        assert_eq!(decoded.to_display_string(), "legacy");
    }
}
