use arookieofcDB::wal::wal_config::WalConfig;
use arookieofcDB::wal::wal_manager::WalManager;
use arookieofcDB::wal::wal_reader::WalReader;

const TEST_DIR: &str = "./data/wal_debug_test";

fn setup_test_dir() {
    let _ = std::fs::remove_dir_all(TEST_DIR);
    std::fs::create_dir_all(TEST_DIR).unwrap();
}

fn teardown_test_dir() {
    let _ = std::fs::remove_dir_all(TEST_DIR);
}

#[test]
fn test_debug_seek_to() {
    setup_test_dir();
    let config = WalConfig::new(TEST_DIR).with_max_file_size(100);
    let mut manager = WalManager::new(config.clone()).unwrap();

    // 写入 20 个条目
    for i in 0..20 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        let seq = manager.write_entry(key.as_bytes(), value.as_bytes()).unwrap();
        println!("Wrote: seq={}, key={}", seq, key);
    }

    // 读取所有条目并打印
    let mut reader = WalReader::new(config.clone()).unwrap();
    println!("\nReading all entries:");
    while let Ok(Some(entry)) = reader.read_entry() {
        println!("Read: seq={}, key={}", entry.seq, String::from_utf8_lossy(&entry.key));
    }

    // 测试 seek_to
    let mut reader2 = WalReader::new(config).unwrap();
    println!("\nTesting seek_to(15):");
    match reader2.seek_to(15) {
        Ok(Some(entry)) => {
            println!("Found: seq={}, key={}", entry.seq, String::from_utf8_lossy(&entry.key));
            assert_eq!(entry.seq, 15);
            assert_eq!(entry.key, b"key14");
        }
        Ok(None) => println!("seek_to returned None"),
        Err(e) => println!("seek_to failed: {:?}", e),
    }

    teardown_test_dir();
}
