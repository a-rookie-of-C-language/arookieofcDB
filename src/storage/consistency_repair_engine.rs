use std::io;
use super::repair_target::RepairTarget;
use super::repair_report::RepairReport;
use super::repair_summary::RepairSummary;

pub trait ConsistencyRepairEngine {
    fn repair_consistency(&mut self, _target: RepairTarget) -> io::Result<RepairReport> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "consistency repair is not supported by this engine",
        ))
    }

    fn last_repair_summary(&self) -> Option<RepairSummary> {
        None
    }
}