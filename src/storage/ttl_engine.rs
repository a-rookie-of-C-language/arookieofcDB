use std::io;
use crate::key_codec::KeyEncoding;
use super::ttl_state::TtlState;

pub trait TtlEngine {
    fn expire(&mut self, _key: &KeyEncoding, _seconds: u64) -> io::Result<bool> {
        Ok(false)
    }

    fn ttl(&mut self, _key: &KeyEncoding) -> io::Result<TtlState> {
        Ok(TtlState::NotFound)
    }
}