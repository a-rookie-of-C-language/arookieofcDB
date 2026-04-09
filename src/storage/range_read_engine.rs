use std::io;
use crate::key_codec::KeyEncoding;

pub trait RangeReadEngine {
    fn range(
        &mut self,
        _start: &KeyEncoding,
        _end: &KeyEncoding,
    ) -> io::Result<Vec<(KeyEncoding, Vec<u8>)>> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "range read is not supported by this engine",
        ))
    }
}