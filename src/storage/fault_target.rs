#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultTarget {
    CacheOnly,
    DiskOnly,
}