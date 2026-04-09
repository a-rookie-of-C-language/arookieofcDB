use super::consistency_diff::ConsistencyDiff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsistencyReport {
    pub cache_keys: usize,
    pub disk_keys: usize,
    pub only_in_cache: usize,
    pub only_in_disk: usize,
    pub value_mismatches: usize,
    pub samples: Vec<ConsistencyDiff>,
}

impl ConsistencyReport {
    pub fn total_issues(&self) -> usize {
        self.only_in_cache + self.only_in_disk + self.value_mismatches
    }
}