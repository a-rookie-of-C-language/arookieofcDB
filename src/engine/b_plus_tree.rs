use std::sync::{Arc, Mutex};
use crate::key_codec::KeyEncoding;
use crate::storage::buffer_pool::BufferPoolManager;
use crate::storage::disk_manager::PageId;
use super::b_plus_tree_page_type::BPlusTreePageType;
use super::b_plus_tree_page_header::BPlusTreePageHeader;
use super::leaf_page_accessor::LeafPageAccessor;
use super::internal_page_accessor::InternalPageAccessor;
use super::b_plus_tree_metadata::BPlusTreeMetadata;

#[derive(Debug)]
pub struct BPlusTree {
    bpm: Arc<Mutex<BufferPoolManager>>,
}

impl BPlusTree {
    pub fn new(bpm: Arc<Mutex<BufferPoolManager>>) -> Self {
        {
            let mut guard = bpm.lock().unwrap();
            let exists = if let Ok(Some(page)) = guard.fetch_page(0) {
                let meta = BPlusTreeMetadata::deserialize(&page.read().unwrap().data);
                guard.unpin_page(0, false);
                meta.root_page_id != 0
            } else {
                false
            };

            if !exists {
                let _meta_wrapper = guard.new_page().unwrap().expect("Failed to create meta page");
                let root_wrapper = guard.new_page().unwrap().expect("Failed to create root page");
                let root_id = root_wrapper.read().unwrap().id;

                {
                    let mut rw = root_wrapper.write().unwrap();
                    let header = BPlusTreePageHeader {
                        page_type: BPlusTreePageType::LeafPage,
                        parent_page_id: 0,
                        key_count: 0,
                        next_page_id: 0,
                    };
                    header.serialize(&mut rw.data);
                }

                let meta_wrapper = guard.fetch_page(0).unwrap().unwrap();
                {
                    let mut mw = meta_wrapper.write().unwrap();
                    let meta = BPlusTreeMetadata { root_page_id: root_id };
                    meta.serialize(&mut mw.data);
                }

                guard.unpin_page(0, true);
                guard.unpin_page(root_id, true);
            }
        }
        Self { bpm }
    }

    fn get_root_id(&self) -> PageId {
        let mut guard = self.bpm.lock().unwrap();
        let page = guard.fetch_page(0).unwrap().expect("Meta page missing");
        let meta = BPlusTreeMetadata::deserialize(&page.read().unwrap().data);
        guard.unpin_page(0, false);
        meta.root_page_id
    }

    pub fn insert(&mut self, key: KeyEncoding, value: Vec<u8>) {
        let root_id = self.get_root_id();
        self.insert_recursive(root_id, key, value);
    }

    fn insert_recursive(&mut self, page_id: PageId, key: KeyEncoding, value: Vec<u8>) {
        let child_id;
        {
            let mut guard = self.bpm.lock().unwrap();
            let page_wrapper = guard.fetch_page(page_id).unwrap().unwrap();
            
            let page_type = {
                let p = page_wrapper.read().unwrap();
                BPlusTreePageType::from_u8(p.data[0]).unwrap()
            };

            match page_type {
                BPlusTreePageType::LeafPage => {
                    let mut rw = page_wrapper.write().unwrap();
                    let mut accessor = LeafPageAccessor::new(&mut rw.data);
                    accessor.insert(&key, &value);
                    guard.unpin_page(page_id, true);
                    return;
                }
                BPlusTreePageType::InternalPage => {
                    let mut rw = page_wrapper.write().unwrap();
                    let accessor = InternalPageAccessor::new(&mut rw.data);
                    child_id = accessor.get_child_id(&key);
                    guard.unpin_page(page_id, false);
                }
                _ => {
                    guard.unpin_page(page_id, false);
                    return;
                }
            }
        }
        self.insert_recursive(child_id, key, value);
    }

    pub fn get(&self, key: &KeyEncoding) -> Option<Vec<u8>> {
        let root_id = self.get_root_id();
        self.get_recursive(root_id, key)
    }

    fn get_recursive(&self, page_id: PageId, key: &KeyEncoding) -> Option<Vec<u8>> {
        let child_id;
        {
            let mut guard = self.bpm.lock().unwrap();
            let page_wrapper = guard.fetch_page(page_id).unwrap().unwrap();
            
            let page_type = {
                let p = page_wrapper.read().unwrap();
                BPlusTreePageType::from_u8(p.data[0]).unwrap()
            };

            match page_type {
                BPlusTreePageType::LeafPage => {
                    let rw = page_wrapper.read().unwrap();
                    let mut data_clone = rw.data.clone();
                    let accessor = LeafPageAccessor::new(&mut data_clone);
                    let res = accessor.get(key);
                    guard.unpin_page(page_id, false);
                    return res;
                }
                BPlusTreePageType::InternalPage => {
                    let mut data_clone = page_wrapper.read().unwrap().data.clone();
                    let accessor = InternalPageAccessor::new(&mut data_clone);
                    child_id = accessor.get_child_id(key);
                    guard.unpin_page(page_id, false);
                }
                _ => {
                    guard.unpin_page(page_id, false);
                    return None;
                }
            }
        }
        self.get_recursive(child_id, key)
    }

    pub fn len(&self) -> usize {
        0
    }

    pub fn delete(&mut self, _key: &KeyEncoding) -> Option<Vec<u8>> {
        None
    }

    pub fn range_query(&self, _start: &KeyEncoding, _end: &KeyEncoding) -> Vec<(KeyEncoding, Vec<u8>)> {
        Vec::new()
    }

    pub fn entries(&self) -> Vec<(KeyEncoding, Vec<u8>)> {
        Vec::new()
    }
}
