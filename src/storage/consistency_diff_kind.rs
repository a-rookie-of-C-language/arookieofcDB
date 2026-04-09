#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsistencyDiffKind {
    OnlyInCache,
    OnlyInDisk,
    ValueMismatch,
}