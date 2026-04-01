use std::fs;
use std::io;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::key_codec::KeyEncoding;

pub fn first_arg(args: &str) -> Option<&str> {
    args.split_whitespace().next()
}

pub fn parse_key(raw: Option<&str>, missing_msg: &str) -> io::Result<KeyEncoding> {
    let raw = raw.ok_or_else(|| invalid_input(missing_msg))?;
    Ok(KeyEncoding::from_input(raw))
}

pub fn parse_i64(raw: Option<&str>, missing_msg: &str) -> io::Result<i64> {
    let raw = raw.ok_or_else(|| invalid_input(missing_msg))?;
    raw.parse::<i64>()
        .map_err(|_| invalid_input(&format!("invalid i64: {raw}")))
}

pub fn invalid_input(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg.to_string())
}

pub(crate) fn file_size_or_zero(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub(crate) fn file_size_or_zero_opt(path: Option<&Path>) -> u64 {
    path.map(file_size_or_zero).unwrap_or(0)
}

pub(crate) fn file_mtime_unix(path: &Path) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    modified
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

pub(crate) fn file_mtime_unix_opt(path: Option<&Path>) -> Option<u64> {
    path.and_then(file_mtime_unix)
}
