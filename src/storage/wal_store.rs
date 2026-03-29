use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::{EngineStats, StorageEngine, SyncPolicy, TtlState};
use crate::engine::BPlusTree;
use crate::key_codec::KeyEncoding;

const OP_SET_V1: u8 = 1;
const OP_DELETE_V1: u8 = 2;
const OP_SET_V2: u8 = 3;
const OP_DELETE_V2: u8 = 4;
const OP_EXPIRE_V2: u8 = 5;

const SNAPSHOT_MAGIC_V1: &[u8; 9] = b"ARDBSNAP1";
const SNAPSHOT_MAGIC_V2: &[u8; 9] = b"ARDBSNAP2";
const SNAPSHOT_MAGIC_V3: &[u8; 9] = b"ARDBSNAP3";

#[derive(Debug)]
pub struct WalStore {
    tree: BPlusTree,
    wal_path: PathBuf,
    snapshot_path: PathBuf,
    wal_file: File,
    sync_policy: SyncPolicy,
    expires: HashMap<KeyEncoding, u64>,
    stats: EngineStats,
}

impl WalStore {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let wal_path = path.as_ref().to_path_buf();
        if let Some(parent) = wal_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let snapshot_path = snapshot_path_for(&wal_path);
        let mut tree = BPlusTree::new();
        let mut expires = HashMap::new();

        if snapshot_path.exists() {
            load_snapshot(&snapshot_path, &mut tree, &mut expires)?;
        }

        if wal_path.exists() {
            replay_wal(&wal_path, &mut tree, &mut expires)?;
        }

        purge_expired_state(&mut tree, &mut expires);

        let wal_file = open_wal_append(&wal_path)?;

        Ok(Self {
            tree,
            wal_path,
            snapshot_path,
            wal_file,
            sync_policy: SyncPolicy::Always,
            expires,
            stats: EngineStats::default(),
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

    pub fn len(&mut self) -> usize {
        self.tree.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    pub fn get(&mut self, key: &KeyEncoding) -> Option<&[u8]> {
        self.tree.get(key)
    }

    pub fn get_with_expiry_flag(&mut self, key: &KeyEncoding) -> (Option<Vec<u8>>, bool) {
        let had_deadline = self.expires.contains_key(key);
        let existed_before = self.tree.get(key).is_some();
        self.purge_if_expired(key);
        let value = self.tree.get(key).map(|v| v.to_vec());
        let expired_now = had_deadline && existed_before && value.is_none();
        (value, expired_now)
    }

    pub fn get_with_expiry_ref(&mut self, key: &KeyEncoding) -> (Option<&[u8]>, bool) {
        let had_deadline = self.expires.contains_key(key);
        let existed_before = self.tree.get(key).is_some();
        self.purge_if_expired(key);
        let value = self.tree.get(key);
        let expired_now = had_deadline && existed_before && value.is_none();
        (value, expired_now)
    }

    pub fn set(&mut self, key: KeyEncoding, value: Vec<u8>) -> io::Result<()> {
        self.stats.writes += 1;
        self.stats.disk_writes += 1;

        append_set_v2(&mut self.wal_file, &key, &value)?;
        self.stats.wal_appends += 1;

        self.after_wal_append()?;
        self.expires.remove(&key);
        self.tree.insert(key, value);
        Ok(())
    }

    pub fn delete(&mut self, key: &KeyEncoding) -> io::Result<Option<Vec<u8>>> {
        self.stats.deletes += 1;

        let deleted = self.tree.delete(key);
        if deleted.is_some() {
            self.stats.disk_writes += 1;
            append_delete_v2(&mut self.wal_file, key)?;
            self.stats.wal_appends += 1;
            self.after_wal_append()?;
        }
        self.expires.remove(key);
        Ok(deleted)
    }

    pub fn range_query(
        &self,
        start: &KeyEncoding,
        end: &KeyEncoding,
    ) -> Vec<(KeyEncoding, Vec<u8>)> {
        self.tree.range_query(start, end)
    }

    pub fn entries(&mut self) -> Vec<(KeyEncoding, Vec<u8>)> {
        self.purge_all_expired();
        self.tree.entries()
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.sync_wal_data()
    }

    pub fn checkpoint(&mut self) -> io::Result<()> {
        self.purge_all_expired();
        write_snapshot(&self.snapshot_path, &self.tree, &self.expires)?;
        self.stats.disk_writes += 1;
        self.rotate_wal()
    }

    fn rotate_wal(&mut self) -> io::Result<()> {
        self.wal_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.wal_path)?;
        self.sync_wal_data()?;
        self.wal_file = open_wal_append(&self.wal_path)?;
        Ok(())
    }

    fn sync_wal_data(&mut self) -> io::Result<()> {
        self.wal_file.sync_data()?;
        self.stats.fsync_count += 1;
        Ok(())
    }

    fn after_wal_append(&mut self) -> io::Result<()> {
        match self.sync_policy {
            SyncPolicy::Always => self.sync_wal_data(),
            SyncPolicy::Manual => Ok(()),
        }
    }

    fn purge_if_expired(&mut self, key: &KeyEncoding) {
        let now_ms = now_unix_ms();
        let expired = self
            .expires
            .get(key)
            .is_some_and(|deadline_ms| now_ms >= *deadline_ms);
        if expired {
            self.expires.remove(key);
            let _ = self.tree.delete(key);
        }
    }

    fn purge_all_expired(&mut self) {
        purge_expired_state(&mut self.tree, &mut self.expires);
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

fn purge_expired_state(tree: &mut BPlusTree, expires: &mut HashMap<KeyEncoding, u64>) {
    let now_ms = now_unix_ms();
    let expired_keys = expires
        .iter()
        .filter_map(|(k, deadline_ms)| {
            if now_ms >= *deadline_ms {
                Some(k.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for key in expired_keys {
        expires.remove(&key);
        let _ = tree.delete(&key);
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

fn write_key_blob<W: Write>(writer: &mut W, key: &KeyEncoding) -> io::Result<()> {
    let key_bytes = key.encode();
    writer.write_all(&(key_bytes.len() as u32).to_le_bytes())?;
    writer.write_all(&key_bytes)
}

fn read_key_blob_opt<R: Read>(reader: &mut R) -> io::Result<Option<KeyEncoding>> {
    let Some(key_len) = read_u32_opt(reader)? else {
        return Ok(None);
    };
    let Some(key_raw) = read_vec_opt(reader, key_len as usize)? else {
        return Ok(None);
    };

    let key = KeyEncoding::decode(&key_raw).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "invalid encoded key in wal/snapshot")
    })?;

    Ok(Some(key))
}

fn append_set_v2(file: &mut File, key: &KeyEncoding, value: &[u8]) -> io::Result<()> {
    file.write_all(&[OP_SET_V2])?;
    write_key_blob(file, key)?;
    file.write_all(&(value.len() as u32).to_le_bytes())?;
    file.write_all(value)
}

fn append_delete_v2(file: &mut File, key: &KeyEncoding) -> io::Result<()> {
    file.write_all(&[OP_DELETE_V2])?;
    write_key_blob(file, key)
}

fn append_expire_v2(file: &mut File, key: &KeyEncoding, deadline_ms: u64) -> io::Result<()> {
    file.write_all(&[OP_EXPIRE_V2])?;
    write_key_blob(file, key)?;
    file.write_all(&deadline_ms.to_le_bytes())
}

fn replay_wal(
    path: &Path,
    tree: &mut BPlusTree,
    expires: &mut HashMap<KeyEncoding, u64>,
) -> io::Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    loop {
        let Some(op) = read_u8_opt(&mut reader)? else {
            break;
        };

        match op {
            OP_SET_V1 => {
                let Some(key) = read_i64_opt(&mut reader)? else {
                    break;
                };
                let Some(value_len) = read_u32_opt(&mut reader)? else {
                    break;
                };
                let Some(value) = read_vec_opt(&mut reader, value_len as usize)? else {
                    break;
                };
                let key = KeyEncoding::Int(key);
                tree.insert(key.clone(), value);
                expires.remove(&key);
            }
            OP_DELETE_V1 => {
                let Some(key) = read_i64_opt(&mut reader)? else {
                    break;
                };
                let key = KeyEncoding::Int(key);
                tree.delete(&key);
                expires.remove(&key);
            }
            OP_SET_V2 => {
                let Some(key) = read_key_blob_opt(&mut reader)? else {
                    break;
                };
                let Some(value_len) = read_u32_opt(&mut reader)? else {
                    break;
                };
                let Some(value) = read_vec_opt(&mut reader, value_len as usize)? else {
                    break;
                };
                tree.insert(key.clone(), value);
                expires.remove(&key);
            }
            OP_DELETE_V2 => {
                let Some(key) = read_key_blob_opt(&mut reader)? else {
                    break;
                };
                tree.delete(&key);
                expires.remove(&key);
            }
            OP_EXPIRE_V2 => {
                let Some(key) = read_key_blob_opt(&mut reader)? else {
                    break;
                };
                let Some(deadline_ms) = read_u64_opt(&mut reader)? else {
                    break;
                };
                if tree.get(&key).is_some() {
                    expires.insert(key, deadline_ms);
                }
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

fn write_snapshot(
    path: &Path,
    tree: &BPlusTree,
    expires: &HashMap<KeyEncoding, u64>,
) -> io::Result<()> {
    let entries = tree.entries();
    let tmp = path.with_extension("snap.tmp");

    let file = File::create(&tmp)?;
    let mut writer = BufWriter::new(file);

    writer.write_all(SNAPSHOT_MAGIC_V3)?;
    writer.write_all(&(entries.len() as u64).to_le_bytes())?;

    for (key, value) in entries {
        write_key_blob(&mut writer, &key)?;
        writer.write_all(&(value.len() as u32).to_le_bytes())?;
        writer.write_all(&value)?;
    }

    let ttl_entries = expires
        .iter()
        .filter_map(|(k, deadline_ms)| {
            if tree.get(k).is_some() {
                Some((k, deadline_ms))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    writer.write_all(&(ttl_entries.len() as u64).to_le_bytes())?;
    for (key, deadline_ms) in ttl_entries {
        write_key_blob(&mut writer, key)?;
        writer.write_all(&deadline_ms.to_le_bytes())?;
    }

    writer.flush()?;
    drop(writer);

    fs::rename(tmp, path)?;
    Ok(())
}

fn load_snapshot(
    path: &Path,
    tree: &mut BPlusTree,
    expires: &mut HashMap<KeyEncoding, u64>,
) -> io::Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; SNAPSHOT_MAGIC_V1.len()];
    reader.read_exact(&mut magic)?;

    let version = if &magic == SNAPSHOT_MAGIC_V3 {
        3
    } else if &magic == SNAPSHOT_MAGIC_V2 {
        2
    } else if &magic == SNAPSHOT_MAGIC_V1 {
        1
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid snapshot magic",
        ));
    };

    let count = read_u64(&mut reader)? as usize;
    for _ in 0..count {
        let key = if version >= 2 {
            read_key_blob(&mut reader)?
        } else {
            KeyEncoding::Int(read_i64(&mut reader)?)
        };

        let value_len = read_u32(&mut reader)? as usize;
        let mut value = vec![0u8; value_len];
        reader.read_exact(&mut value)?;
        tree.insert(key, value);
    }

    if version >= 3 {
        let ttl_count = read_u64(&mut reader)? as usize;
        for _ in 0..ttl_count {
            let key = read_key_blob(&mut reader)?;
            let deadline_ms = read_u64(&mut reader)?;
            if tree.get(&key).is_some() {
                expires.insert(key, deadline_ms);
            }
        }
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

fn read_u64_opt<R: Read>(reader: &mut R) -> io::Result<Option<u64>> {
    let mut bytes = [0u8; 8];
    if read_exact_or_eof(reader, &mut bytes)?.is_none() {
        return Ok(None);
    }
    Ok(Some(u64::from_le_bytes(bytes)))
}

fn read_vec_opt<R: Read>(reader: &mut R, len: usize) -> io::Result<Option<Vec<u8>>> {
    let mut v = vec![0u8; len];
    if read_exact_or_eof(reader, &mut v)?.is_none() {
        return Ok(None);
    }
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

fn read_key_blob(reader: &mut BufReader<File>) -> io::Result<KeyEncoding> {
    let len = read_u32(reader)? as usize;
    let mut key_raw = vec![0u8; len];
    reader.read_exact(&mut key_raw)?;
    KeyEncoding::decode(&key_raw)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid encoded key in snapshot"))
}

impl StorageEngine for WalStore {
    fn engine_name(&self) -> &'static str {
        "wal_bptree"
    }

    fn len(&mut self) -> usize {
        self.purge_all_expired();
        WalStore::len(self)
    }

    fn get(&mut self, key: &KeyEncoding) -> Option<&[u8]> {
        self.stats.reads += 1;
        self.stats.disk_reads += 1;
        self.purge_if_expired(key);
        WalStore::get(self, key)
    }

    fn get_disk_only(&mut self, key: &KeyEncoding) -> Option<Vec<u8>> {
        self.stats.reads += 1;
        self.stats.disk_reads += 1;
        let (value, expired_now) = self.get_with_expiry_flag(key);
        if expired_now {
            self.stats.ttl_expired_on_disk += 1;
        }
        value
    }

    fn set(&mut self, key: KeyEncoding, value: Vec<u8>) -> io::Result<()> {
        WalStore::set(self, key, value)
    }

    fn delete(&mut self, key: &KeyEncoding) -> io::Result<Option<Vec<u8>>> {
        WalStore::delete(self, key)
    }

    fn range_query(
        &self,
        start: &KeyEncoding,
        end: &KeyEncoding,
    ) -> Vec<(KeyEncoding, Vec<u8>)> {
        WalStore::range_query(self, start, end)
    }

    fn sync(&mut self) -> io::Result<()> {
        WalStore::sync(self)
    }

    fn checkpoint(&mut self) -> io::Result<()> {
        WalStore::checkpoint(self)
    }

    fn expire(&mut self, key: &KeyEncoding, seconds: u64) -> io::Result<bool> {
        self.purge_if_expired(key);
        if self.tree.get(key).is_none() {
            return Ok(false);
        }

        let deadline_ms = now_unix_ms().saturating_add(seconds.saturating_mul(1000));
        append_expire_v2(&mut self.wal_file, key, deadline_ms)?;
        self.stats.wal_appends += 1;
        self.stats.disk_writes += 1;
        self.after_wal_append()?;

        self.expires.insert(key.clone(), deadline_ms);
        Ok(true)
    }

    fn ttl(&mut self, key: &KeyEncoding) -> io::Result<TtlState> {
        self.purge_if_expired(key);

        if self.tree.get(key).is_none() {
            return Ok(TtlState::NotFound);
        }

        let Some(deadline_ms) = self.expires.get(key) else {
            return Ok(TtlState::NoExpire);
        };

        let now_ms = now_unix_ms();
        if *deadline_ms <= now_ms {
            return Ok(TtlState::NotFound);
        }

        let remain = ((*deadline_ms - now_ms) / 1000) as i64;
        Ok(TtlState::Seconds(remain))
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

    fn stats(&self) -> EngineStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::key_codec::KeyEncoding;
    use crate::storage::{StorageEngine, TtlState};

    use super::{OP_SET_V2, SyncPolicy, WalStore};

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
            store.set(KeyEncoding::Int(1), b"v1".to_vec()).expect("set 1");
            store
                .set(KeyEncoding::Raw("user:2".to_string()), b"v2".to_vec())
                .expect("set 2");
            store.delete(&KeyEncoding::Int(1)).expect("delete 1");
            store.sync().expect("sync");
        }

        {
            let mut store = WalStore::open(&wal_path).expect("re-open store");
            assert_eq!(store.get(&KeyEncoding::Int(1)), None);
            assert_eq!(
                store.get(&KeyEncoding::Raw("user:2".to_string())),
                Some("v2".as_bytes())
            );
            assert_eq!(store.len(), 1);
        }

        cleanup(&wal_path);
    }

    #[test]
    fn checkpoint_compacts_wal_and_restores_state() {
        let wal_path = unique_wal_path();

        {
            let mut store = WalStore::open(&wal_path).expect("open store");
            store.set(KeyEncoding::Int(1), b"v1".to_vec()).expect("set 1");
            store
                .set(KeyEncoding::Raw("k2".to_string()), b"v2".to_vec())
                .expect("set 2");
            store.delete(&KeyEncoding::Int(1)).expect("delete 1");
            store.checkpoint().expect("checkpoint");

            let wal_len = std::fs::metadata(store.wal_path())
                .expect("wal metadata")
                .len();
            assert_eq!(wal_len, 0);
            assert!(store.snapshot_path().exists());
        }

        {
            let mut store = WalStore::open(&wal_path).expect("re-open from snapshot");
            assert_eq!(store.get(&KeyEncoding::Int(1)), None);
            assert_eq!(
                store.get(&KeyEncoding::Raw("k2".to_string())),
                Some("v2".as_bytes())
            );
            assert_eq!(store.len(), 1);

            store
                .set(KeyEncoding::Float(3.5f64.to_bits()), b"v3".to_vec())
                .expect("set 3");
        }

        {
            let mut store = WalStore::open(&wal_path).expect("re-open second time");
            assert_eq!(
                store.get(&KeyEncoding::Raw("k2".to_string())),
                Some("v2".as_bytes())
            );
            assert_eq!(
                store.get(&KeyEncoding::Float(3.5f64.to_bits())),
                Some("v3".as_bytes())
            );
            assert_eq!(store.len(), 2);
        }

        cleanup(&wal_path);
    }

    #[test]
    fn truncated_wal_tail_is_ignored_on_replay() {
        let wal_path = unique_wal_path();

        {
            let mut store = WalStore::open(&wal_path).expect("open store");
            store
                .set(KeyEncoding::Int(11), b"ok".to_vec())
                .expect("set");
            store.set_sync_policy(SyncPolicy::Manual);
            store.sync().expect("sync");
        }

        {
            let mut f = OpenOptions::new()
                .append(true)
                .open(&wal_path)
                .expect("open wal append");
            f.write_all(&[OP_SET_V2]).expect("write partial op");
            f.sync_data().expect("sync partial");
        }

        let mut store = WalStore::open(&wal_path).expect("re-open with truncated tail");
        assert_eq!(store.get(&KeyEncoding::Int(11)), Some("ok".as_bytes()));

        cleanup(&wal_path);
    }

    #[test]
    fn ttl_survives_restart() {
        let wal_path = unique_wal_path();
        let key = KeyEncoding::Raw("ttl:k".to_string());

        {
            let mut store = WalStore::open(&wal_path).expect("open store");
            store.set(key.clone(), b"v".to_vec()).expect("set");
            assert!(store.expire(&key, 30).expect("expire"));
        }

        {
            let mut restarted = WalStore::open(&wal_path).expect("re-open");
            assert_eq!(restarted.get(&key), Some("v".as_bytes()));
            match restarted.ttl(&key).expect("ttl") {
                TtlState::Seconds(sec) => assert!(sec >= 1),
                other => panic!("unexpected ttl state: {other:?}"),
            }
        }

        cleanup(&wal_path);
    }

    #[test]
    fn expired_key_does_not_reappear_after_restart() {
        let wal_path = unique_wal_path();
        let key = KeyEncoding::Raw("ttl:e".to_string());

        {
            let mut store = WalStore::open(&wal_path).expect("open store");
            store.set(key.clone(), b"v".to_vec()).expect("set");
            assert!(store.expire(&key, 1).expect("expire"));
            store.sync().expect("sync");
        }

        thread::sleep(Duration::from_millis(1100));

        {
            let mut restarted = WalStore::open(&wal_path).expect("re-open");
            assert_eq!(restarted.get(&key), None);
            assert_eq!(restarted.ttl(&key).expect("ttl"), TtlState::NotFound);
        }

        cleanup(&wal_path);
    }
}






