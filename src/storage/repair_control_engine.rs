use std::io;
use super::repair_mode::RepairMode;

pub trait RepairControlEngine {
    fn set_repair_mode(&mut self, _mode: RepairMode) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "repair mode is not supported by this engine",
        ))
    }

    fn repair_mode(&self) -> Option<RepairMode> {
        None
    }
}