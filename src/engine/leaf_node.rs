use crate::key_codec::KeyEncoding;

#[derive(Debug, Clone)]
pub(crate) struct LeafNode {
    pub(crate) keys: Vec<KeyEncoding>,
    pub(crate) values: Vec<Vec<u8>>,
    pub(crate) next: Option<usize>,
}
