use std::io;
use crate::key_codec::KeyEncoding;
use super::fault_target::FaultTarget;

pub trait FaultInjectionEngine {
    fn inject_fault(
        &mut self,
        _target: FaultTarget,
        _key: KeyEncoding,
        _value: Vec<u8>,
    ) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "fault injection is not supported by this engine",
        ))
    }
}