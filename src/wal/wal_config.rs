use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct WalConfig {
    pub wal_dir: PathBuf,
    pub max_file_size: u64,
    pub checkpoint_interval: u64,
    pub max_retained_files: usize,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            wal_dir: PathBuf::from("./data/wal"),
            max_file_size: 1024 * 1024 * 64,
            checkpoint_interval: 1000,
            max_retained_files: 10,
        }
    }
}

impl WalConfig {
    pub fn new(wal_dir: &str) -> Self {
        Self {
            wal_dir: PathBuf::from(wal_dir),
            ..Self::default()
        }
    }

    pub fn with_max_file_size(mut self, max_file_size: u64) -> Self {
        self.max_file_size = max_file_size;
        self
    }

    pub fn with_checkpoint_interval(mut self, interval: u64) -> Self {
        self.checkpoint_interval = interval;
        self
    }

    pub fn with_max_retained_files(mut self, count: usize) -> Self {
        self.max_retained_files = count;
        self
    }
}
