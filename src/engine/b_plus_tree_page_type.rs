#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BPlusTreePageType {
    LeafPage = 0,
    InternalPage = 1,
    MetadataPage = 2,
}

impl BPlusTreePageType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::LeafPage),
            1 => Some(Self::InternalPage),
            2 => Some(Self::MetadataPage),
            _ => None,
        }
    }
}
