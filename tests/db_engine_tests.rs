use std::fs;

use arookieofcDB::storage::db_engine::DbEngine;
use arookieofcDB::wal::wal_config::WalConfig;

const TEST_DIR: &str = "./data/db_engine_test";

fn setup_test_dir() {
    let _ = fs::remove_dir_all(TEST_DIR);
    fs::create_dir_all(TEST_DIR).unwrap();
}

fn teardown_test_dir() {
    let _ = fs::remove_dir_all(TEST_DIR);
}

#[test]
fn test_db_engine_set_get() {
    setup_test_dir();
    let config = WalConfig::new(TEST_DIR);
    let engine = DbEngine::new(config).unwrap();

    engine.set(b"key1", b"value1").unwrap();
    engine.set(b"key2", b"value2").unwrap();

    assert_eq!(engine.get(b"key1"), Some(b"value1".to_vec()));
    assert_eq!(engine.get(b"key2"), Some(b"value2".to_vec()));
    assert_eq!(engine.get(b"nonexistent"), None);

    teardown_test_dir();
}

#[test]
fn test_db_engine_delete() {
    setup_test_dir();
    let config = WalConfig::new(TEST_DIR);
    let engine = DbEngine::new(config).unwrap();

    engine.set(b"key1", b"value1").unwrap();
    assert!(engine.contains_key(b"key1"));

    let result = engine.delete(b"key1").unwrap();
    assert!(result);
    assert!(!engine.contains_key(b"key1"));

    let result = engine.delete(b"nonexistent").unwrap();
    assert!(!result);

    teardown_test_dir();
}

#[test]
fn test_db_engine_persistence() {
    setup_test_dir();
    let config = WalConfig::new(TEST_DIR);
    
    {
        // 创建第一个引擎实例并写入数据
        let engine1 = DbEngine::new(config.clone()).unwrap();
        engine1.set(b"persist_key", b"persist_value").unwrap();
        engine1.set(b"another_key", b"another_value").unwrap();
        engine1.create_checkpoint().unwrap();
    } // engine1 在这里被销毁，释放文件锁

    // 创建第二个引擎实例，验证数据持久化
    let engine2 = DbEngine::new(config).unwrap();
    
    assert_eq!(engine2.get(b"persist_key"), Some(b"persist_value".to_vec()));
    assert_eq!(engine2.get(b"another_key"), Some(b"another_value".to_vec()));

    teardown_test_dir();
}

#[test]
fn test_db_engine_recovery_from_wal() {
    setup_test_dir();
    let config = WalConfig::new(TEST_DIR);
    
    {
        // 创建引擎并写入数据
        let engine1 = DbEngine::new(config.clone()).unwrap();
        for i in 0..10 {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            engine1.set(key.as_bytes(), value.as_bytes()).unwrap();
        }
    } // engine1 在这里被销毁，释放文件锁

    // 不创建 checkpoint，直接创建新引擎验证从 WAL 恢复
    let engine2 = DbEngine::new(config).unwrap();
    
    assert_eq!(engine2.len(), 10);
    for i in 0..10 {
        let key = format!("key{}", i);
        let expected_value = format!("value{}", i).as_bytes().to_vec();
        assert_eq!(engine2.get(key.as_bytes()), Some(expected_value));
    }

    teardown_test_dir();
}

#[test]
fn test_db_engine_checkpoint_cleanup() {
    setup_test_dir();
    let config = WalConfig::new(TEST_DIR).with_max_file_size(100);
    let engine = DbEngine::new(config).unwrap();

    // 写入足够多的数据触发文件滚动
    for i in 0..50 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        engine.set(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let initial_files = fs::read_dir(TEST_DIR).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().unwrap_or_default() == "wal")
        .count();

    engine.create_checkpoint().unwrap();

    let remaining_files = fs::read_dir(TEST_DIR).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().unwrap_or_default() == "wal")
        .count();

    assert!(remaining_files < initial_files);

    teardown_test_dir();
}
