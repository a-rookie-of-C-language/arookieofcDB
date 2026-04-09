use std::io;

pub trait CachePolicyEngine {
    fn set_cache_policy(&mut self, _policy: &str) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cache policy is not supported by this engine",
        ))
    }

    fn cache_policy(&self) -> Option<&'static str> {
        None
    }
}