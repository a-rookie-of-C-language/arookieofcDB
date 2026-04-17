use std::collections::HashMap;
use std::fs;

use arookieofcDB::wal::checkpoint::{Checkpoint, CheckpointManager};
use arookieofcDB::wal::wal_config::WalConfig;
use arookieofcDB::wal::wal_manager::WalManager;
use arookieofcDB::wal::wal_reader::WalReader;

fn setup_test_dir(name: &str) -> String {
    let dir = format!("./data/wal_test_{}", name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn teardown_test_dir(name: &str) {
    let dir = format!("./data/wal_test_{}", name);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_wal_file_rotation() {
    let test_dir = setup_test_dir("rotation");
    let config = WalConfig::new(&test_dir).with_max_file_size(100);
    let mut manager = WalManager::new(config).unwrap();

    for i in 0..20 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        manager.write_entry(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let files = fs::read_dir(&test_dir).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().unwrap_or_default() == "wal")
        .count();
    
    assert!(files > 1, "Should have multiple files after rotation");
    teardown_test_dir("rotation");
}

#[test]
fn test_wal_write_read_consistency() {
    let test_dir = setup_test_dir("consistency");
    let config = WalConfig::new(&test_dir);
    let mut manager = WalManager::new(config.clone()).unwrap();

    let test_data = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
        (b"key3".to_vec(), b"value3".to_vec()),
    ];

    for (key, value) in &test_data {
        manager.write_entry(key, value).unwrap();
    }

    manager.flush_buffer().unwrap();

    let mut reader = WalReader::new(config).unwrap();
    let mut count = 0;

    for result in reader.iter() {
        let entry = result.unwrap();
        assert_eq!(entry.key, test_data[count].0);
        assert_eq!(entry.value, test_data[count].1);
        count += 1;
    }

    assert_eq!(count, 3);
    teardown_test_dir("consistency");
}

#[test]
fn test_wal_multiple_files_read() {
    let test_dir = setup_test_dir("multi_read");
    let config = WalConfig::new(&test_dir).with_max_file_size(100);
    let mut manager = WalManager::new(config.clone()).unwrap();

    for i in 0..30 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        manager.write_entry(key.as_bytes(), value.as_bytes()).unwrap();
    }

    manager.flush_buffer().unwrap();

    let mut reader = WalReader::new(config).unwrap();
    let mut count = 0;

    for result in reader.iter() {
        let entry = result.unwrap();
        let expected_key = format!("key{}", count);
        let expected_value = format!("value{}", count);
        assert_eq!(entry.key, expected_key.as_bytes());
        assert_eq!(entry.value, expected_value.as_bytes());
        count += 1;
    }

    assert_eq!(count, 30);
    teardown_test_dir("multi_read");
}

#[test]
fn test_checkpoint_save_load() {
    let test_dir = setup_test_dir("checkpoint_save_load");
    let config = WalConfig::new(&test_dir);
    let cp_manager = CheckpointManager::new(config.clone());

    let mut data = HashMap::new();
    data.insert(b"key1".to_vec(), b"value1".to_vec());
    data.insert(b"key2".to_vec(), b"value2".to_vec());
    data.insert(b"key3".to_vec(), b"value3".to_vec());

    let checkpoint = Checkpoint::new(100, data.clone());
    cp_manager.save_checkpoint(&checkpoint).unwrap();

    let loaded = cp_manager.load_latest_checkpoint().unwrap().unwrap();
    
    assert_eq!(loaded.sequence(), 100);
    assert_eq!(loaded.entry_count(), 3);
    
    for (key, value) in data {
        assert_eq!(loaded.data().get(&key).unwrap(), &value);
    }

    teardown_test_dir("checkpoint_save_load");
}

#[test]
fn test_checkpoint_multiple_cleanup() {
    let test_dir = setup_test_dir("checkpoint_cleanup");
    let config = WalConfig::new(&test_dir);
    let cp_manager = CheckpointManager::new(config.clone());

    for i in 0..5 {
        let mut data = HashMap::new();
        data.insert(format!("key{}", i).as_bytes().to_vec(), b"value".to_vec());
        let checkpoint = Checkpoint::new(i as u64 * 100, data);
        cp_manager.save_checkpoint(&checkpoint).unwrap();
    }

    let checkpoints = cp_manager.list_checkpoints().unwrap();
    assert_eq!(checkpoints.len(), 6);

    let removed = cp_manager.cleanup_old_checkpoints(2).unwrap();
    assert_eq!(removed, 3);

    let remaining = cp_manager.list_checkpoints().unwrap();
    assert_eq!(remaining.len(), 3);

    teardown_test_dir("checkpoint_cleanup");
}

#[test]
fn test_wal_cleanup_old_files() {
    let test_dir = setup_test_dir("wal_cleanup");
    let config = WalConfig::new(&test_dir).with_max_file_size(100);
    let mut manager = WalManager::new(config).unwrap();

    for i in 0..50 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        manager.write_entry(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let initial_files = fs::read_dir(&test_dir).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().unwrap_or_default() == "wal")
        .count();

    manager.cleanup_old_files(45).unwrap();

    let remaining_files = fs::read_dir(&test_dir).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().unwrap_or_default() == "wal")
        .count();

    assert!(remaining_files < initial_files);
    teardown_test_dir("wal_cleanup");
}

#[test]
fn test_wal_reader_seek_to() {
    let test_dir = setup_test_dir("seek_to");
    let config = WalConfig::new(&test_dir).with_max_file_size(100);
    let mut manager = WalManager::new(config.clone()).unwrap();

    for i in 0..30 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        manager.write_entry(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let mut reader = WalReader::new(config).unwrap();
    let entry = reader.seek_to(15).unwrap().unwrap();
    let expected_key = format!("key{}", 14);
    assert_eq!(entry.seq, 15);
    assert_eq!(entry.key, expected_key.as_bytes());

    teardown_test_dir("seek_to");
}

#[test]
fn test_wal_reader_reset() {
    let test_dir = setup_test_dir("reset");
    let config = WalConfig::new(&test_dir);
    let mut manager = WalManager::new(config.clone()).unwrap();

    for i in 0..5 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        manager.write_entry(key.as_bytes(), value.as_bytes()).unwrap();
    }

    manager.flush_buffer().unwrap();

    let mut reader = WalReader::new(config).unwrap();
    
    for _ in 0..3 {
        reader.read_entry().unwrap();
    }

    reader.reset().unwrap();
    let entry = reader.read_entry().unwrap().unwrap();
    
    assert_eq!(entry.seq, 1);
    assert_eq!(entry.key, b"key0");

    teardown_test_dir("reset");
}

#[test]
fn test_checkpoint_integration_with_wal() {
    let test_dir = setup_test_dir("integration");
    let config = WalConfig::new(&test_dir);
    let mut manager = WalManager::new(config.clone()).unwrap();
    let cp_manager = CheckpointManager::new(config.clone());

    for i in 0..10 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        manager.write_entry(key.as_bytes(), value.as_bytes()).unwrap();
    }

    let mut data = HashMap::new();
    for i in 0..10 {
        data.insert(format!("key{}", i).as_bytes().to_vec(), format!("value{}", i).as_bytes().to_vec());
    }
    let checkpoint = Checkpoint::new(10, data);
    cp_manager.save_checkpoint(&checkpoint).unwrap();

    manager.cleanup_old_files(10).unwrap();

    let files = fs::read_dir(&test_dir).unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().unwrap_or_default() == "wal")
        .count();
    
    assert!(files <= 1);

    teardown_test_dir("integration");
}
