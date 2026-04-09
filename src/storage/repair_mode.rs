#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepairMode {
    Off,
    Read,
    Write,
    Always,
}

impl RepairMode {
    pub fn from_input(raw: &str) -> Option<Self> {
        match raw.to_ascii_lowercase().as_str() {
            "off" => Some(Self::Off),
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            "always" => Some(Self::Always),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Read => "read",
            Self::Write => "write",
            Self::Always => "always",
        }
    }
}