#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyEncoding {
    Raw(String),
    Int(i64),
    Float(u64),
}

const MAGIC: &[u8; 3] = b"KE1";
const TAG_RAW: u8 = 0;
const TAG_INT: u8 = 1;
const TAG_FLOAT: u8 = 2;

impl KeyEncoding {
    pub fn from_input(input: &str) -> Self {
        if let Ok(v) = input.parse::<i64>() {
            return Self::Int(v);
        }

        let looks_float = input.contains('.') || input.contains('e') || input.contains('E');
        if looks_float {
            if let Ok(v) = input.parse::<f64>() {
                if v.is_finite() {
                    return Self::Float(v.to_bits());
                }
            }
        }

        Self::Raw(input.to_string())
    }

    pub fn to_display_string(&self) -> String {
        match self {
            Self::Raw(v) => v.clone(),
            Self::Int(v) => v.to_string(),
            Self::Float(bits) => f64::from_bits(*bits).to_string(),
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);

        match self {
            Self::Raw(s) => {
                let bytes = s.as_bytes();
                out.push(TAG_RAW);
                out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                out.extend_from_slice(bytes);
            }
            Self::Int(v) => {
                out.push(TAG_INT);
                out.extend_from_slice(&v.to_le_bytes());
            }
            Self::Float(bits) => {
                out.push(TAG_FLOAT);
                out.extend_from_slice(&bits.to_le_bytes());
            }
        }

        out
    }

    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 || &bytes[..3] != MAGIC {
            return None;
        }

        match bytes[3] {
            TAG_RAW => {
                if bytes.len() < 8 {
                    return None;
                }
                let mut len_bytes = [0u8; 4];
                len_bytes.copy_from_slice(&bytes[4..8]);
                let len = u32::from_le_bytes(len_bytes) as usize;
                if bytes.len() != 8 + len {
                    return None;
                }
                let raw = String::from_utf8(bytes[8..].to_vec()).ok()?;
                Some(Self::Raw(raw))
            }
            TAG_INT => {
                if bytes.len() != 12 {
                    return None;
                }
                let mut int_bytes = [0u8; 8];
                int_bytes.copy_from_slice(&bytes[4..12]);
                Some(Self::Int(i64::from_le_bytes(int_bytes)))
            }
            TAG_FLOAT => {
                if bytes.len() != 12 {
                    return None;
                }
                let mut float_bytes = [0u8; 8];
                float_bytes.copy_from_slice(&bytes[4..12]);
                Some(Self::Float(u64::from_le_bytes(float_bytes)))
            }
            _ => None,
        }
    }
}

impl Ord for KeyEncoding {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use KeyEncoding::{Float, Int, Raw};

        match (self, other) {
            (Int(a), Int(b)) => a.cmp(b),
            (Float(a), Float(b)) => f64::from_bits(*a)
                .partial_cmp(&f64::from_bits(*b))
                .unwrap_or(std::cmp::Ordering::Equal),
            (Raw(a), Raw(b)) => a.cmp(b),
            (Int(_), _) => std::cmp::Ordering::Less,
            (Float(_), Raw(_)) => std::cmp::Ordering::Less,
            (Float(_), Int(_)) => std::cmp::Ordering::Greater,
            (Raw(_), _) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for KeyEncoding {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::KeyEncoding;

    #[test]
    fn detect_int() {
        assert!(matches!(KeyEncoding::from_input("100"), KeyEncoding::Int(100)));
    }

    #[test]
    fn detect_float() {
        match KeyEncoding::from_input("3.14") {
            KeyEncoding::Float(bits) => assert_eq!(f64::from_bits(bits), 3.14),
            other => panic!("unexpected encoding: {other:?}"),
        }
    }

    #[test]
    fn raw_fallback() {
        assert!(matches!(KeyEncoding::from_input("hello"), KeyEncoding::Raw(_)));
    }

    #[test]
    fn round_trip() {
        let items = [
            KeyEncoding::Raw("abc".to_string()),
            KeyEncoding::Int(42),
            KeyEncoding::Float(1.5f64.to_bits()),
        ];

        for item in items {
            let encoded = item.encode();
            let decoded = KeyEncoding::decode(&encoded).expect("decode key");
            assert_eq!(decoded, item);
        }
    }
}
