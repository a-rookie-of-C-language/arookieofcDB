use std::io;
use crate::key_codec::KeyEncoding;

pub trait KvEngine {
    fn engine_name(&self) -> &'static str;
    fn len(&mut self) -> usize;
    fn get(&mut self, key: &KeyEncoding) -> Option<&[u8]>;
    fn set(&mut self, key: KeyEncoding, value: Vec<u8>) -> io::Result<()>;
    fn delete(&mut self, key: &KeyEncoding) -> io::Result<Option<Vec<u8>>>;
}