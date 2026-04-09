use std::path::Path;

pub trait StoragePathIntrospection {
    fn wal_path(&self) -> Option<&Path> {
        None
    }

    fn snapshot_path(&self) -> Option<&Path> {
        None
    }
}