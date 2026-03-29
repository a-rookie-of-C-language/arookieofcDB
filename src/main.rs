mod commands;
mod engine;
mod storage;
mod value_codec;
mod key_codec;

use crate::commands::CommandRegistry;
use crate::commands::CommandSignal;
use crate::storage::{HybridStore, MemoryStore, StorageEngine, WalStore};
use std::io::{self, Write};
use std::time::Instant;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> io::Result<()> {
    let mut store = build_engine()?;
    let registry = CommandRegistry::new();

    println!("arookieofcDB CLI v{}", APP_VERSION);
    println!("type `help` to see commands");

    let stdin = io::stdin();
    loop {
        print!("arookieofcDB> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }

        let started_at = Instant::now();
        let mut should_exit = false;

        match registry.execute_line(store.as_mut(), &line) {
            Ok(output) => {
                if let Some(message) = output.message {
                    println!("{message}");
                }
                match output.signal {
                    CommandSignal::Continue => {}
                    CommandSignal::Exit => should_exit = true,
                    CommandSignal::SwitchEngine(mode) => {
                        store = build_engine_by_mode(&mode)?;
                        println!("ok (engine switched to {mode})");
                    }
                }
            }
            Err(err) => eprintln!("error: {err}"),
        }

        println!("(elapsed: {})", format_elapsed(started_at.elapsed()));
        if should_exit {
            break;
        }
    }

    Ok(())
}

fn build_engine() -> io::Result<Box<dyn StorageEngine>> {
    let mode = std::env::var("AROOKIE_ENGINE").unwrap_or_else(|_| String::from("memory"));
    build_engine_by_mode(&mode)
}

fn build_engine_by_mode(mode: &str) -> io::Result<Box<dyn StorageEngine>> {
    match mode.to_ascii_lowercase().as_str() {
        "wal" | "disk" | "bptree" => Ok(Box::new(WalStore::open("data/wal.log")?)),
        "hybrid" => Ok(Box::new(HybridStore::open("data/wal.log")?)),
        _ => Ok(Box::new(MemoryStore::new())),
    }
}

fn format_elapsed(duration: std::time::Duration) -> String {
    if duration.as_millis() >= 1 {
        format!("{} ms", duration.as_millis())
    } else {
        format!("{} us", duration.as_micros())
    }
}
