use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::{Arc, RwLock};

use super::disk_manager::{DiskManager, Page, PageId, PAGE_SIZE};

pub type FrameId = usize;

pub struct BufferPoolManager {
    disk_manager: DiskManager,
    frames: Vec<Arc<RwLock<Page>>>,
    page_table: HashMap<PageId, FrameId>,
    free_list: VecDeque<FrameId>,
    replacer: VecDeque<FrameId>, // Simple LRU Queue
}

impl BufferPoolManager {
    pub fn new(pool_size: usize, disk_manager: DiskManager) -> Self {
        let mut frames = Vec::with_capacity(pool_size);
        let mut free_list = VecDeque::with_capacity(pool_size);
        for i in 0..pool_size {
            frames.push(Arc::new(RwLock::new(Page::new(0))));
            free_list.push_back(i);
        }

        Self {
            disk_manager,
            frames,
            page_table: HashMap::new(),
            free_list,
            replacer: VecDeque::new(),
        }
    }

    pub fn fetch_page(&mut self, page_id: PageId) -> io::Result<Option<Arc<RwLock<Page>>>> {
        if let Some(&frame_id) = self.page_table.get(&page_id) {
            let page = Arc::clone(&self.frames[frame_id]);
            page.write().unwrap().pin_count += 1;
            
            self.replacer.retain(|&f| f != frame_id);
            self.replacer.push_back(frame_id);
            
            return Ok(Some(page));
        }

        let frame_id = match self.get_victim_frame()? {
            Some(id) => id,
            None => return Ok(None), 
        };

        let mut page_data = [0; PAGE_SIZE];
        self.disk_manager.read_page(page_id, &mut page_data)?;

        self.page_table.insert(page_id, frame_id);
        self.replacer.push_back(frame_id);

        let page = Arc::clone(&self.frames[frame_id]);
        {
            let mut p = page.write().unwrap();
            p.id = page_id;
            p.data = page_data;
            p.pin_count = 1;
            p.is_dirty = false;
        }

        Ok(Some(page))
    }

    fn get_victim_frame(&mut self) -> io::Result<Option<FrameId>> {
        if let Some(frame_id) = self.free_list.pop_front() {
            return Ok(Some(frame_id));
        }

        let mut victim_idx = None;
        for (i, &frame_id) in self.replacer.iter().enumerate() {
            let page = self.frames[frame_id].read().unwrap();
            if page.pin_count == 0 {
                victim_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = victim_idx {
            let frame_id = self.replacer.remove(idx).unwrap();
            
            let old_page_id;
            let is_dirty;
            let mut page_data = [0; PAGE_SIZE];
            {
                let p = self.frames[frame_id].read().unwrap();
                old_page_id = p.id;
                is_dirty = p.is_dirty;
                if is_dirty {
                    page_data.copy_from_slice(&p.data);
                }
            }

            if is_dirty {
                self.disk_manager.write_page(old_page_id, &page_data)?;
                self.frames[frame_id].write().unwrap().is_dirty = false;
            }

            self.page_table.remove(&old_page_id);
            return Ok(Some(frame_id));
        }

        Ok(None)
    }

    pub fn unpin_page(&mut self, page_id: PageId, is_dirty_flag: bool) -> bool {
        if let Some(&frame_id) = self.page_table.get(&page_id) {
            let mut p = self.frames[frame_id].write().unwrap();
            if p.pin_count == 0 {
                return false;
            }
            p.pin_count -= 1;
            if is_dirty_flag {
                p.is_dirty = true;
            }
            return true;
        }
        false
    }

    pub fn new_page(&mut self) -> io::Result<Option<Arc<RwLock<Page>>>> {
        let frame_id = match self.get_victim_frame()? {
            Some(id) => id,
            None => return Ok(None),
        };

        let page_id = self.disk_manager.allocate_page();
        
        self.page_table.insert(page_id, frame_id);
        self.replacer.push_back(frame_id);

        let page = Arc::clone(&self.frames[frame_id]);
        {
            let mut p = page.write().unwrap();
            p.id = page_id;
            p.data = [0; PAGE_SIZE];
            p.pin_count = 1;
            p.is_dirty = true; 
        }

        Ok(Some(page))
    }

    pub fn flush_page(&mut self, page_id: PageId) -> io::Result<bool> {
        if let Some(&frame_id) = self.page_table.get(&page_id) {
            let mut p = self.frames[frame_id].write().unwrap();
            if p.is_dirty {
                self.disk_manager.write_page(p.id, &p.data)?;
                p.is_dirty = false;
            }
            return Ok(true);
        }
        Ok(false)
    }

    pub fn flush_all_pages(&mut self) -> io::Result<()> {
        for (&_page_id, &frame_id) in self.page_table.iter() {
            let mut p = self.frames[frame_id].write().unwrap();
            if p.is_dirty {
                self.disk_manager.write_page(p.id, &p.data)?;
                p.is_dirty = false;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_buffer_pool_eviction() {
        let db_path = "test_buffer_pool.ibd";
        let _ = fs::remove_file(db_path);
        
        let dm = DiskManager::new(db_path).unwrap();
        let mut bpm = BufferPoolManager::new(3, dm);

        // Fetch 3 pages (fills the pool)
        let page0 = bpm.new_page().unwrap().unwrap();
        let page1 = bpm.new_page().unwrap().unwrap();
        let page2 = bpm.new_page().unwrap().unwrap();

        assert_eq!(page0.read().unwrap().id, 0);
        assert_eq!(page1.read().unwrap().id, 1);
        assert_eq!(page2.read().unwrap().id, 2);

        // Unpin them so they can be evicted
        bpm.unpin_page(0, true);
        bpm.unpin_page(1, false);
        bpm.unpin_page(2, false);

        // Fetch 4th page, should evict page 0 (LRU)
        let page3 = bpm.new_page().unwrap().unwrap();
        assert_eq!(page3.read().unwrap().id, 3);
        bpm.unpin_page(3, true);

        // Fetch page 0 again from disk
        let fetched_page0 = bpm.fetch_page(0).unwrap().unwrap();
        assert_eq!(fetched_page0.read().unwrap().id, 0);
        
        // Clean up
        let _ = fs::remove_file(db_path);
    }
}
