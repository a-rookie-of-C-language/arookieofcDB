use crate::key_codec::KeyEncoding;
use super::consistency_diff_kind::ConsistencyDiffKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyDiff {
    pub key: KeyEncoding,
    pub kind: ConsistencyDiffKind,
}