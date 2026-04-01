use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::key_codec::KeyEncoding;
use crate::storage::{
    CacheCapacityEngine, CachePolicyEngine, DiskReadEngine, KvEngine,
    StorageStatsIntrospection, TtlEngine, TtlState, HybridStore, MemoryStore, WalStore,
};
use crate::value_codec::StringEncoding;

fn unique_wal_path(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("arookieofcdb-{prefix}-{nanos}.wal"))
}

fn cleanup(path: &Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("snap"));
    let _ = std::fs::remove_file(path.with_extension("snap.tmp"));
}

fn s(input: &str) -> Vec<u8> {
    StringEncoding::from_input(input).encode()
}

fn decoded(store: &mut dyn KvEngine, key: &KeyEncoding) -> Option<String> {
    store
        .get(key)
        .map(|raw| StringEncoding::decode(raw).to_display_string())
}

#[test]
fn memory_is_not_persistent_across_instances() {
    let key = KeyEncoding::Raw("u:1".to_string());

    let mut store = MemoryStore::new();
    store.set(key.clone(), s("alice")).expect("set in memory");
    assert_eq!(decoded(&mut store, &key), Some("alice".to_string()));

    let mut restarted = MemoryStore::new();
    assert_eq!(decoded(&mut restarted, &key), None);
}

#[test]
fn wal_restart_preserves_last_write_and_delete() {
    let wal_path = unique_wal_path("wal-consistency");
    let k1 = KeyEncoding::Raw("user:1".to_string());
    let k2 = KeyEncoding::Raw("user:2".to_string());

    {
        let mut store = WalStore::open(&wal_path).expect("open wal");
        store.set(k1.clone(), s("v1")).expect("set v1");
        store.set(k1.clone(), s("v2")).expect("update v2");
        store.set(k2.clone(), s("gone")).expect("set k2");
        store.delete(&k2).expect("delete k2");
        store.sync().expect("sync");
    }

    {
        let mut restarted = WalStore::open(&wal_path).expect("reopen wal");
        assert_eq!(decoded(&mut restarted, &k1), Some("v2".to_string()));
        assert_eq!(decoded(&mut restarted, &k2), None);
    }

    cleanup(&wal_path);
}

#[test]
fn hybrid_restart_preserves_last_write_and_delete() {
    let wal_path = unique_wal_path("hybrid-consistency");
    let k1 = KeyEncoding::Raw("order:1".to_string());
    let k2 = KeyEncoding::Raw("order:2".to_string());

    {
        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        store.set(k1.clone(), s("init")).expect("set init");
        store.set(k1.clone(), s("final")).expect("update final");
        store.set(k2.clone(), s("to-delete")).expect("set k2");
        store.delete(&k2).expect("delete k2");
        store.sync().expect("sync");
    }

    {
        let mut restarted = HybridStore::open(&wal_path).expect("reopen hybrid");
        assert_eq!(decoded(&mut restarted, &k1), Some("final".to_string()));
        assert_eq!(decoded(&mut restarted, &k2), None);
    }

    cleanup(&wal_path);
}

#[test]
fn hybrid_cache_stays_consistent_after_update_and_delete() {
    let wal_path = unique_wal_path("hybrid-cache");
    let key = KeyEncoding::Raw("cache:key".to_string());

    let mut store = HybridStore::open(&wal_path).expect("open hybrid");
    store.set(key.clone(), s("v1")).expect("set v1");

    assert_eq!(decoded(&mut store, &key), Some("v1".to_string()));

    store.set(key.clone(), s("v2")).expect("update v2");
    assert_eq!(decoded(&mut store, &key), Some("v2".to_string()));

    store.delete(&key).expect("delete key");
    assert_eq!(decoded(&mut store, &key), None);

    cleanup(&wal_path);
}

#[test]
fn wal_recovers_from_truncated_tail_after_checkpoint() {
    let wal_path = unique_wal_path("wal-crash-checkpoint");
    let k1 = KeyEncoding::Raw("cp:1".to_string());
    let k2 = KeyEncoding::Raw("cp:2".to_string());
    let k3 = KeyEncoding::Raw("cp:3".to_string());

    {
        let mut store = WalStore::open(&wal_path).expect("open wal");
        store.set(k1.clone(), s("a")).expect("set k1");
        store.set(k2.clone(), s("b")).expect("set k2");
        store.checkpoint().expect("checkpoint");
        store.set(k3.clone(), s("c")).expect("set k3 after checkpoint");
        store.sync().expect("sync");
    }

    {
        let mut f = OpenOptions::new()
            .append(true)
            .open(&wal_path)
            .expect("open wal append");
        f.write_all(&[3]).expect("write partial op");
        f.sync_data().expect("sync partial");
    }

    {
        let mut restarted = WalStore::open(&wal_path).expect("reopen wal");
        assert_eq!(decoded(&mut restarted, &k1), Some("a".to_string()));
        assert_eq!(decoded(&mut restarted, &k2), Some("b".to_string()));
        assert_eq!(decoded(&mut restarted, &k3), Some("c".to_string()));
    }

    cleanup(&wal_path);
}

#[test]
fn hybrid_recovers_from_truncated_wal_tail() {
    let wal_path = unique_wal_path("hybrid-crash-tail");
    let key = KeyEncoding::Raw("hy:1".to_string());

    {
        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        store.set(key.clone(), s("ok")).expect("set");
        store.sync().expect("sync");
    }

    {
        let mut f = OpenOptions::new()
            .append(true)
            .open(&wal_path)
            .expect("open wal append");
        f.write_all(&[4]).expect("write partial op");
        f.sync_data().expect("sync partial");
    }

    {
        let mut restarted = HybridStore::open(&wal_path).expect("reopen hybrid");
        assert_eq!(decoded(&mut restarted, &key), Some("ok".to_string()));
    }

    cleanup(&wal_path);
}

#[test]
fn wal_ttl_is_persisted_across_restart() {
    let wal_path = unique_wal_path("wal-ttl-restart");
    let key = KeyEncoding::Raw("ttl:wal".to_string());

    {
        let mut store = WalStore::open(&wal_path).expect("open wal");
        store.set(key.clone(), s("v")).expect("set");
        assert!(store.expire(&key, 30).expect("expire"));
        match store.ttl(&key).expect("ttl before restart") {
            TtlState::Seconds(sec) => assert!(sec >= 1),
            other => panic!("unexpected ttl before restart: {other:?}"),
        }
    }

    {
        let mut restarted = WalStore::open(&wal_path).expect("reopen wal");
        assert_eq!(decoded(&mut restarted, &key), Some("v".to_string()));
        match restarted.ttl(&key).expect("ttl after restart") {
            TtlState::Seconds(sec) => assert!(sec >= 1),
            other => panic!("unexpected ttl after restart: {other:?}"),
        }
        let stats = restarted.stats();
        assert!(stats.ttl_loaded_on_startup >= 1);
        assert_eq!(stats.ttl_pruned_on_startup, 0);
    }

    cleanup(&wal_path);
}

#[test]
fn wal_expired_while_down_remains_expired_after_restart() {
    let wal_path = unique_wal_path("wal-ttl-down");
    let key = KeyEncoding::Raw("ttl:down".to_string());

    {
        let mut store = WalStore::open(&wal_path).expect("open wal");
        store.set(key.clone(), s("v")).expect("set");
        assert!(store.expire(&key, 1).expect("expire 1s"));
        store.sync().expect("sync");
    }

    thread::sleep(Duration::from_millis(1100));

    {
        let mut restarted = WalStore::open(&wal_path).expect("reopen wal");
        assert_eq!(decoded(&mut restarted, &key), None);
        assert_eq!(restarted.ttl(&key).expect("ttl after restart"), TtlState::NotFound);
        let stats = restarted.stats();
        assert!(stats.ttl_loaded_on_startup >= 1);
        assert!(stats.ttl_pruned_on_startup >= 1);
    }

    cleanup(&wal_path);
}

#[test]
fn hybrid_get_and_disk_read_stay_consistent_after_update_and_delete() {
    let wal_path = unique_wal_path("hybrid-read-order");
    let key = KeyEncoding::Raw("hy:order:1".to_string());

    let mut store = HybridStore::open(&wal_path).expect("open hybrid");

    store.set(key.clone(), s("v1")).expect("set v1");
    assert_eq!(decoded(&mut store, &key), Some("v1".to_string()));
    assert_eq!(
        store
            .get_disk_only(&key)
            .map(|v| StringEncoding::decode(&v).to_display_string()),
        Some("v1".to_string())
    );

    store.set(key.clone(), s("v2")).expect("set v2");
    assert_eq!(decoded(&mut store, &key), Some("v2".to_string()));
    assert_eq!(
        store
            .get_disk_only(&key)
            .map(|v| StringEncoding::decode(&v).to_display_string()),
        Some("v2".to_string())
    );

    store.delete(&key).expect("delete key");
    assert_eq!(decoded(&mut store, &key), None);
    assert_eq!(store.get_disk_only(&key), None);

    cleanup(&wal_path);
}

#[test]
fn hybrid_cache_miss_backfills_from_disk_consistently() {
    let wal_path = unique_wal_path("hybrid-backfill");
    let key = KeyEncoding::Raw("hy:miss:1".to_string());

    let mut store = HybridStore::open(&wal_path).expect("open hybrid");
    store.set_cache_policy("none").expect("set none");
    store.set(key.clone(), s("from-disk")).expect("set key");
    assert_eq!(store.cache_current_keys(), Some(0));

    store.set_cache_policy("lru").expect("set lru");

    assert_eq!(decoded(&mut store, &key), Some("from-disk".to_string()));
    assert_eq!(store.cache_current_keys(), Some(1));
    assert_eq!(
        store
            .get_disk_only(&key)
            .map(|v| StringEncoding::decode(&v).to_display_string()),
        Some("from-disk".to_string())
    );

    cleanup(&wal_path);
}

#[test]
fn hybrid_ttl_is_persisted_across_restart() {
    let wal_path = unique_wal_path("hybrid-ttl-restart");
    let key = KeyEncoding::Raw("ttl:hybrid".to_string());

    {
        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        store.set(key.clone(), s("v")).expect("set");
        assert!(store.expire(&key, 30).expect("expire"));
    }

    {
        let mut restarted = HybridStore::open(&wal_path).expect("reopen hybrid");
        assert_eq!(decoded(&mut restarted, &key), Some("v".to_string()));
        match restarted.ttl(&key).expect("ttl after restart") {
            TtlState::Seconds(sec) => assert!(sec >= 1),
            other => panic!("unexpected ttl state: {other:?}"),
        }
        let stats = restarted.stats();
        assert!(stats.ttl_loaded_on_startup >= 1);
        assert_eq!(stats.ttl_pruned_on_startup, 0);
    }

    cleanup(&wal_path);
}

#[test]
fn hybrid_expired_while_down_remains_expired_after_restart() {
    let wal_path = unique_wal_path("hybrid-ttl-down");
    let key = KeyEncoding::Raw("ttl:hy-down".to_string());

    {
        let mut store = HybridStore::open(&wal_path).expect("open hybrid");
        store.set(key.clone(), s("v")).expect("set");
        assert!(store.expire(&key, 1).expect("expire"));
        store.sync().expect("sync");
    }

    thread::sleep(Duration::from_millis(1100));

    {
        let mut restarted = HybridStore::open(&wal_path).expect("reopen hybrid");
        assert_eq!(decoded(&mut restarted, &key), None);
        assert_eq!(restarted.ttl(&key).expect("ttl"), TtlState::NotFound);
        let stats = restarted.stats();
        assert!(stats.ttl_loaded_on_startup >= 1);
        assert!(stats.ttl_pruned_on_startup >= 1);
    }

    cleanup(&wal_path);
}

#[test]
fn hybrid_cache_eviction_does_not_break_disk_truth() {
    let wal_path = unique_wal_path("hybrid-cache-evict-disk-truth");
    let k1 = KeyEncoding::Raw("hy:evict:1".to_string());
    let k2 = KeyEncoding::Raw("hy:evict:2".to_string());

    let mut store = HybridStore::open(&wal_path).expect("open hybrid");
    store.set_cache_max_keys(1).expect("cache max 1");
    store.set(k1.clone(), s("v1")).expect("set k1");
    store.set(k2.clone(), s("v2")).expect("set k2");

    assert_eq!(
        store
            .get_disk_only(&k1)
            .map(|v| StringEncoding::decode(&v).to_display_string()),
        Some("v1".to_string())
    );
    assert_eq!(
        store
            .get_disk_only(&k2)
            .map(|v| StringEncoding::decode(&v).to_display_string()),
        Some("v2".to_string())
    );

    cleanup(&wal_path);
}
