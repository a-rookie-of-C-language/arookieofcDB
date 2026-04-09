use std::io;

pub trait DurabilityEngine {
    fn sync(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "durability sync is not supported by this engine",
        ))
    }

    fn checkpoint(&mut self) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "durability checkpoint is not supported by this engine",
        ))
    }
}