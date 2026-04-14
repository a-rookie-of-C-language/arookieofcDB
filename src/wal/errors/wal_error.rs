use std::io;

#[derive(Debug)]
pub enum WalError {
    IOError(io::Error),
    Corrupted(String),
    InvalidMagic,
    UnexpectedEof,
    FileNotFound,
    ChecksumMismatch,
    InvalidHeader(String),
    InvalidPath,
}

impl From<io::Error> for WalError {
    fn from(e: io::Error) -> Self {
        WalError::IOError(e)
    }
}
