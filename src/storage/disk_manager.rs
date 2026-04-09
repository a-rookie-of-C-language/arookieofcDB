use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

pub const PAGE_SIZE: usize = 4096;
pub type PageId = u32;

/// A standard Database Page holding fixed size bytes
pub struct Page {
    pub id: PageId,
    pub data: [u8; PAGE_SIZE],
    pub is_dirty: bool,
    pub pin_count: u32,
}

impl Page {
    pub fn new(id: PageId) -> Self {
        Self {
            id,
            data: [0; PAGE_SIZE],
            is_dirty: false,
            pin_count: 0,
        }
    }
}

pub struct DiskManager {
    file: File,
    next_page_id: PageId,
}

impl DiskManager {
    pub fn new<P: AsRef<Path>>(db_file: P) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(db_file)?;

        let metadata = file.metadata()?;
        let file_size = metadata.len();
        let next_page_id = (file_size / PAGE_SIZE as u64) as PageId;

        Ok(Self { file, next_page_id })
    }

    pub fn read_page(&mut self, page_id: PageId, data: &mut [u8; PAGE_SIZE]) -> io::Result<()> {
        let offset = (page_id as u64) * (PAGE_SIZE as u64);
        
        let file_size = self.file.metadata()?.len();
        if offset >= file_size {
            // Uninitialized page, zero out data
            data.fill(0);
            return Ok(());
        }

        self.file.seek(SeekFrom::Start(offset))?;
        
        // Handle partial reads at EOF
        let mut bytes_read = 0;
        while bytes_read < PAGE_SIZE {
            let n = self.file.read(&mut data[bytes_read..])?;
            if n == 0 {
                break;
            }
            bytes_read += n;
        }

        // Zero out the rest if EOF reached before filling PAGE_SIZE
        if bytes_read < PAGE_SIZE {
            data[bytes_read..].fill(0);
        }

        Ok(())
    }

    pub fn write_page(&mut self, page_id: PageId, data: &[u8; PAGE_SIZE]) -> io::Result<()> {
        let offset = (page_id as u64) * (PAGE_SIZE as u64);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(data)?;
        self.file.flush()?; 
        Ok(())
    }

    pub fn allocate_page(&mut self) -> PageId {
        let page_id = self.next_page_id;
        self.next_page_id += 1;
        page_id
    }
}
