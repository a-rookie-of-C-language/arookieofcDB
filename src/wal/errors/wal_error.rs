use std::fmt;
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

impl fmt::Display for WalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalError::IOError(e) => write!(f, "IO error: {}", e),
            WalError::Corrupted(s) => write!(f, "Corrupted data: {}", s),
            WalError::InvalidMagic => write!(f, "Invalid magic number"),
            WalError::UnexpectedEof => write!(f, "Unexpected end of file"),
            WalError::FileNotFound => write!(f, "File not found"),
            WalError::ChecksumMismatch => write!(f, "Checksum mismatch"),
            WalError::InvalidHeader(s) => write!(f, "Invalid header: {}", s),
            WalError::InvalidPath => write!(f, "Invalid path"),
        }
    }
}

impl From<io::Error> for WalError {
    fn from(e: io::Error) -> Self {
        WalError::IOError(e)
    }
}
