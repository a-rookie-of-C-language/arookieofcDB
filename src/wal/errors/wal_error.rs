use std::io;

pub enum WalError {
    IOError(io::Error),
    Corrupted(String),
    InvalidMagic,
    UnexpectedEof,
    FileNotFound,
    ChecksumMismatch,
}
