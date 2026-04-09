use crate::key_codec::KeyEncoding;

#[derive(Debug, Clone)]
pub(crate) struct InternalNode {
    pub(crate) keys: Vec<KeyEncoding>,
    pub(crate) children: Vec<usize>,
}
