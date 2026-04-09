use std::io;
use super::consistency_report::ConsistencyReport;

pub trait ConsistencyEngine {
    fn verify_consistency(&mut self) -> io::Result<ConsistencyReport> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "consistency verify is not supported by this engine",
        ))
    }
}