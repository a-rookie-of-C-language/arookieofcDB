use super::repair_target::RepairTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepairSummary {
    pub target: RepairTarget,
    pub repaired_only_in_cache: usize,
    pub repaired_only_in_disk: usize,
    pub repaired_value_mismatches: usize,
}

impl RepairSummary {
    pub fn total_repairs(&self) -> usize {
        self.repaired_only_in_cache + self.repaired_only_in_disk + self.repaired_value_mismatches
    }
}