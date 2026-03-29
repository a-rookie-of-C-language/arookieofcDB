use std::collections::HashMap;

use crate::key_codec::KeyEncoding;

#[derive(Debug, Default, Clone)]
pub struct ArookieofcHashTable {
    hash_table: HashMap<KeyEncoding, Vec<u8>>,
}

impl ArookieofcHashTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.hash_table.len()
    }

    pub fn insert(&mut self, key: KeyEncoding, value: Vec<u8>) {
        self.hash_table.insert(key, value);
    }

    pub fn get(&self, key: &KeyEncoding) -> Option<&[u8]> {
        self.hash_table.get(key).map(|v| v.as_slice())
    }

    pub fn remove(&mut self, key: &KeyEncoding) -> Option<Vec<u8>> {
        self.hash_table.remove(key)
    }

    pub fn range_query(&self, start: &KeyEncoding, end: &KeyEncoding) -> Vec<(KeyEncoding, Vec<u8>)> {
        if start > end {
            return Vec::new();
        }

        let mut pairs = self
            .hash_table
            .iter()
            .filter_map(|(k, v)| {
                if k >= start && k <= end {
                    Some((k.clone(), v.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        pairs.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));
        pairs
    }
}
