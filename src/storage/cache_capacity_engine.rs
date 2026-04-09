use std::io;

pub trait CacheCapacityEngine {
    fn set_cache_max_keys(&mut self, _max_keys: usize) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cache max keys is not supported by this engine",
        ))
    }

    fn cache_max_keys(&self) -> Option<usize> {
        None
    }

    fn cache_current_keys(&self) -> Option<usize> {
        None
    }
}