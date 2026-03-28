mod commands;
mod engine;
mod storage;

use crate::commands::CommandSignal;
use crate::commands::CommandRegistry;
use crate::storage::{MemoryStore, StorageEngine, WalStore};
use std::io::{self, Write};

fn main() -> io::Result<()> {
    let mut store = build_engine()?;
    let registry = CommandRegistry::new();

    println!("arookieofcDB CLI");
    println!("type `help` to see commands");

    let stdin = io::stdin();
    loop {
        print!("arookieofcDB> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }

        match registry.execute_line(store.as_mut(), &line) {
            Ok(output) => {
                if let Some(message) = output.message {
                    println!("{message}");
                }

                if output.signal == CommandSignal::Exit {
                    break;
                }
            }
            Err(err) => eprintln!("error: {err}"),
        }
    }

    Ok(())
}

fn build_engine() -> io::Result<Box<dyn StorageEngine>> {
    let mode = std::env::var("AROOKIE_ENGINE").unwrap_or_else(|_| String::from("memory"));
    match mode.to_ascii_lowercase().as_str() {
        "wal" | "disk" | "bptree" => Ok(Box::new(WalStore::open("data/wal.log")?)),
        _ => Ok(Box::new(MemoryStore::new())),
    }
}

