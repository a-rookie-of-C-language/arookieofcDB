use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::engine::BPlusTree;
use super::{StorageEngine, SyncPolicy};

const OP_SET: u8 = 1;
const OP_DELETE: u8 = 2;
const SNAPSHOT_MAGIC: &[u8; 9] = b"ARDBSNAP1";

#[derive(Debug)]
pub struct WalStore {
    tree: BPlusTree,
    wal_path: PathBuf,
    snapshot_path: PathBuf,
    wal_file: File,
    sync_policy: SyncPolicy,
}

impl WalStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let wal_path = path.as_ref().to_path_buf();
        if let Some(parent) = wal_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let snapshot_path = snapshot_path_for(&wal_path);
        let mut tree = BPlusTree::new();

        if snapshot_path.exists() {
            load_snapshot(&snapshot_path, &mut tree)?;
        }

        if wal_path.exists() {
            replay_wal(&wal_path, &mut tree)?;
        }

        let wal_file = open_wal_append(&wal_path)?;

        Ok(Self {
            tree,
            wal_path,
            snapshot_path,
            wal_file,
            sync_policy: SyncPolicy::Always,
        })
    }

    pub fn wal_path(&self) -> &Path {
        &self.wal_path
    }

    pub fn snapshot_path(&self) -> &Path {
        &self.snapshot_path
    }

    pub fn sync_policy(&self) -> SyncPolicy {
        self.sync_policy
    }

    pub fn set_sync_policy(&mut self, policy: SyncPolicy) {
        self.sync_policy = policy;
    }

    pub fn len(&self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    pub fn get(&self, key: i64) -> Option<&[u8]> {
        self.tree.get(key)
    }

    pub fn set(&mut self, key: i64, value: Vec<u8>) -> io::Result<()> {
        append_set(&mut self.wal_file, key, &value)?;
        self.after_wal_append()?;
        self.tree.insert(key, value);
        Ok(())
    }

    pub fn delete(&mut self, key: i64) -> io::Result<Option<Vec<u8>>> {
        let deleted = self.tree.delete(key);
        if deleted.is_some() {
            append_delete(&mut self.wal_file, key)?;
            self.after_wal_append()?;
        }
        Ok(deleted)
    }

    pub fn range_query(&self, start: i64, end: i64) -> Vec<(i64, Vec<u8>)> {
        self.tree.range_query(start, end)
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.wal_file.sync_data()
    }

    pub fn checkpoint(&mut self) -> io::Result<()> {
        write_snapshot(&self.snapshot_path, &self.tree)?;
        self.rotate_wal()
    }

    fn rotate_wal(&mut self) -> io::Result<()> {
        self.wal_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.wal_path)?;
        self.wal_file.sync_data()?;
        self.wal_file = open_wal_append(&self.wal_path)?;
        Ok(())
    }

    fn after_wal_append(&mut self) -> io::Result<()> {
        match self.sync_policy {
            SyncPolicy::Always => self.wal_file.sync_data(),
            SyncPolicy::Manual => Ok(()),
        }
    }
}

fn snapshot_path_for(wal_path: &Path) -> PathBuf {
    wal_path.with_extension("snap")
}

fn open_wal_append(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .read(true)
        .open(path)
}

fn append_set(file: &mut File, key: i64, value: &[u8]) -> io::Result<()> {
    file.write_all(&[OP_SET])?;
    file.write_all(&key.to_le_bytes())?;
    file.write_all(&(value.len() as u32).to_le_bytes())?;
    file.write_all(value)
}

fn append_delete(file: &mut File, key: i64) -> io::Result<()> {
    file.write_all(&[OP_DELETE])?;
    file.write_all(&key.to_le_bytes())
}

fn replay_wal(path: &Path, tree: &mut BPlusTree) -> io::Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    loop {
        let Some(op) = read_u8_opt(&mut reader)? else {
            break;
        };

        let Some(key) = read_i64_opt(&mut reader)? else {
            break;
        };

        match op {
            OP_SET => {
                let Some(value_len) = read_u32_opt(&mut reader)? else {
                    break;
                };
                let Some(value) = read_vec_opt(&mut reader, value_len as usize)? else {
                    break;
                };
                tree.insert(key, value);
            }
            OP_DELETE => {
                tree.delete(key);
            }
            0 => {
                break;
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown wal op code: {other}"),
                ));
            }
        }
    }

    Ok(())
}

fn write_snapshot(path: &Path, tree: &BPlusTree) -> io::Result<()> {
    let entries = tree.range_query(i64::MIN, i64::MAX);
    let tmp = path.with_extension("snap.tmp");

    let file = File::create(&tmp)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(SNAPSHOT_MAGIC)?;
    writer.write_all(&(entries.len() as u64).to_le_bytes())?;

    for (key, value) in entries {
        writer.write_all(&key.to_le_bytes())?;
        writer.write_all(&(value.len() as u32).to_le_bytes())?;
        writer.write_all(&value)?;
    }

    writer.flush()?;
    drop(writer);

    fs::rename(tmp, path)?;
    Ok(())
}

fn load_snapshot(path: &Path, tree: &mut BPlusTree) -> io::Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; SNAPSHOT_MAGIC.len()];
    reader.read_exact(&mut magic)?;
    if &magic != SNAPSHOT_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid snapshot magic",
        ));
    }

    let count = read_u64(&mut reader)? as usize;
    for _ in 0..count {
        let key = read_i64(&mut reader)?;
        let value_len = read_u32(&mut reader)? as usize;
        let mut value = vec![0u8; value_len];
        reader.read_exact(&mut value)?;
        tree.insert(key, value);
    }

    Ok(())
}

fn read_exact_or_eof<R: Read>(reader: &mut R, buf: &mut [u8]) -> io::Result<Option<()>> {
    match reader.read_exact(buf) {
        Ok(()) => Ok(Some(())),
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => Ok(None),
        Err(err) => Err(err),
    }
}

fn read_u8_opt<R: Read>(reader: &mut R) -> io::Result<Option<u8>> {
    let mut b = [0u8; 1];
    if read_exact_or_eof(reader, &mut b)?.is_none() {
        return Ok(None);
    }
    Ok(Some(b[0]))
}

fn read_i64_opt<R: Read>(reader: &mut R) -> io::Result<Option<i64>> {
    let mut bytes = [0u8; 8];
    if read_exact_or_eof(reader, &mut bytes)?.is_none() {
        return Ok(None);
    }
    Ok(Some(i64::from_le_bytes(bytes)))
}

fn read_u32_opt<R: Read>(reader: &mut R) -> io::Result<Option<u32>> {
    let mut bytes = [0u8; 4];
    if read_exact_or_eof(reader, &mut bytes)?.is_none() {
        return Ok(None);
    }
    Ok(Some(u32::from_le_bytes(bytes)))
}

fn read_vec_opt<R: Read>(reader: &mut R, len: usize) -> io::Result<Option<Vec<u8>>> {
    let mut v = vec![0u8; len];
    if read_exact_or_eof(reader, &mut v)?.is_none() {}
    Ok(Some(v))
}

fn read_i64(reader: &mut BufReader<File>) -> io::Result<i64> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(i64::from_le_bytes(bytes))
}

fn read_u32(reader: &mut BufReader<File>) -> io::Result<u32> {
    let mut bytes = [0u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_u64(reader: &mut BufReader<File>) -> io::Result<u64> {
    let mut bytes = [0u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

impl StorageEngine for WalStore {
    fn engine_name(&self) -> &'static str {
        "wal_bptree"
    }

    fn len(&self) -> usize {
        WalStore::len(self)
    }

    fn get(&self, key: i64) -> Option<&[u8]> {
        WalStore::get(self, key)
    }

    fn set(&mut self, key: i64, value: Vec<u8>) -> io::Result<()> {
        WalStore::set(self, key, value)
    }

    fn delete(&mut self, key: i64) -> io::Result<Option<Vec<u8>>> {
        WalStore::delete(self, key)
    }

    fn range_query(&self, start: i64, end: i64) -> Vec<(i64, Vec<u8>)> {
        WalStore::range_query(self, start, end)
    }

    fn sync(&mut self) -> io::Result<()> {
        WalStore::sync(self)
    }

    fn checkpoint(&mut self) -> io::Result<()> {
        WalStore::checkpoint(self)
    }

    fn sync_policy(&self) -> SyncPolicy {
        WalStore::sync_policy(self)
    }

    fn set_sync_policy(&mut self, policy: SyncPolicy) {
        WalStore::set_sync_policy(self, policy);
    }

    fn wal_path(&self) -> Option<&Path> {
        Some(WalStore::wal_path(self))
    }

    fn snapshot_path(&self) -> Option<&Path> {
        Some(WalStore::snapshot_path(self))
    }
}
#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{OP_SET, SyncPolicy, WalStore};

    fn unique_wal_path() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        std::env::temp_dir().join(format!("arookieofcdb-test-{nanos}.wal"))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("snap"));
        let _ = std::fs::remove_file(path.with_extension("snap.tmp"));
    }

    #[test]
    fn replay_from_wal_after_restart() {
        let wal_path = unique_wal_path();

        {
            let mut store = WalStore::open(&wal_path).expect("open store");
            store.set(1, b"v1".to_vec()).expect("set 1");
            store.set(2, b"v2".to_vec()).expect("set 2");
            store.delete(1).expect("delete 1");
            store.sync().expect("sync");
        }

        {
            let store = WalStore::open(&wal_path).expect("re-open store");
            assert_eq!(store.get(1), None);
            assert_eq!(store.get(2), Some("v2".as_bytes()));
            assert_eq!(store.len(), 1);
        }

        cleanup(&wal_path);
    }

    #[test]
    fn checkpoint_compacts_wal_and_restores_state() {
        let wal_path = unique_wal_path();

        {
            let mut store = WalStore::open(&wal_path).expect("open store");
            store.set(1, b"v1".to_vec()).expect("set 1");
            store.set(2, b"v2".to_vec()).expect("set 2");
            store.delete(1).expect("delete 1");
            store.checkpoint().expect("checkpoint");

            let wal_len = std::fs::metadata(store.wal_path())
                .expect("wal metadata")
                .len();
            assert_eq!(wal_len, 0);
            assert!(store.snapshot_path().exists());
        }

        {
            let mut store = WalStore::open(&wal_path).expect("re-open from snapshot");
            assert_eq!(store.get(1), None);
            assert_eq!(store.get(2), Some("v2".as_bytes()));
            assert_eq!(store.len(), 1);

            store.set(3, b"v3".to_vec()).expect("set 3");
        }

        {
            let store = WalStore::open(&wal_path).expect("re-open second time");
            assert_eq!(store.get(2), Some("v2".as_bytes()));
            assert_eq!(store.get(3), Some("v3".as_bytes()));
            assert_eq!(store.len(), 2);
        }

        cleanup(&wal_path);
    }

    #[test]
    fn truncated_wal_tail_is_ignored_on_replay() {
        let wal_path = unique_wal_path();

        {
            let mut store = WalStore::open(&wal_path).expect("open store");
            store.set(11, b"ok".to_vec()).expect("set");
            store.set_sync_policy(SyncPolicy::Manual);
            store.sync().expect("sync");
        }

        {
            let mut f = OpenOptions::new()
                .append(true)
                .open(&wal_path)
                .expect("open wal append");
            f.write_all(&[OP_SET]).expect("write partial op");
            f.sync_data().expect("sync partial");
        }

        let store = WalStore::open(&wal_path).expect("re-open with truncated tail");
        assert_eq!(store.get(11), Some("ok".as_bytes()));

        cleanup(&wal_path);
    }
}





