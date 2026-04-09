use crate::commands::builtin::utils::{file_mtime_unix_opt, file_size_or_zero_opt};
use crate::storage::{
    CacheCapacityEngine, CachePolicyEngine, ConsistencyEngine,
    ConsistencyRepairEngine, DiskReadEngine, DurabilityEngine, EngineStats,
    FaultInjectionEngine, KvEngine, RangeReadEngine, RepairControlEngine, RepairTarget,
    StorageEngine, StorageIntrospection, TtlEngine,
};
use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandSignal {
    Continue,
    Exit,
    SwitchEngine(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub message: Option<String>,
    pub signal: CommandSignal,
}

impl CommandOutput {
    pub fn none() -> Self {
        Self {
            message: None,
            signal: CommandSignal::Continue,
        }
    }

    pub fn message(message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            signal: CommandSignal::Continue,
        }
    }

    pub fn with_signal(message: Option<String>, signal: CommandSignal) -> Self {
        Self { message, signal }
    }

    pub fn switch_engine(mode: impl Into<String>) -> Self {
        let mode = mode.into();
        Self {
            message: Some(format!("switching engine to {mode}")),
            signal: CommandSignal::SwitchEngine(mode),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSnapshot {
    pub engine: &'static str,
    pub len: usize,
    pub stats: EngineStats,
    pub wal_path: String,
    pub wal_bytes: u64,
    pub snapshot_path: String,
    pub snapshot_bytes: u64,
    pub snapshot_mtime_unix: String,
    pub cache_policy: String,
    pub cache_max_keys: String,
    pub cache_current_keys: String,
    pub repair_mode: String,
    pub inconsistency_total: String,
    pub inconsistency_only_in_cache: String,
    pub inconsistency_only_in_disk: String,
    pub inconsistency_value_mismatch: String,
    pub last_repair_target: String,
    pub last_repair_total: String,
    pub last_repair_only_in_cache: String,
    pub last_repair_only_in_disk: String,
    pub last_repair_value_mismatch: String,
}

pub struct CommandContext<'a> {
    store: &'a mut dyn StorageEngine,
}

impl<'a> CommandContext<'a> {
    pub fn kv(&mut self) -> &mut dyn KvEngine {
        self.store
    }

    pub fn ttl(&mut self) -> &mut dyn TtlEngine {
        self.store
    }

    pub fn durability(&mut self) -> &mut dyn DurabilityEngine {
        self.store
    }

    pub fn disk_read(&mut self) -> &mut dyn DiskReadEngine {
        self.store
    }

    pub fn range_read(&mut self) -> &mut dyn RangeReadEngine {
        self.store
    }

    pub fn consistency(&mut self) -> &mut dyn ConsistencyEngine {
        self.store
    }

    pub fn repair(&mut self) -> &mut dyn ConsistencyRepairEngine {
        self.store
    }

    pub fn repair_mode(&mut self) -> &mut dyn RepairControlEngine {
        self.store
    }

    pub fn fault(&mut self) -> &mut dyn FaultInjectionEngine {
        self.store
    }

    pub fn cache_limits(&mut self) -> &mut dyn CacheCapacityEngine {
        self.store
    }

    pub fn cache_config(&mut self) -> &mut dyn CachePolicyEngine {
        self.store
    }

    pub fn inspect(&mut self) -> &dyn StorageIntrospection {
        self.store
    }

    pub fn status_report(&mut self) -> io::Result<StatusSnapshot> {
        let introspection = self.inspect();
        let wal_bytes = file_size_or_zero_opt(introspection.wal_path());
        let snapshot_bytes = file_size_or_zero_opt(introspection.snapshot_path());
        let snapshot_mtime_unix = file_mtime_unix_opt(introspection.snapshot_path())
            .map(|ts: u64| ts.to_string())
            .unwrap_or_else(|| String::from("none"));
        let wal_path = introspection
            .wal_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| String::from("none"));
        let snapshot_path = introspection
            .snapshot_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| String::from("none"));
        let stats = introspection.stats();

        let cache_limits = self.cache_limits();
        let cache_max_keys = cache_limits
            .cache_max_keys()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("none"));
        let cache_current_keys = cache_limits
            .cache_current_keys()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("none"));
        let cache_policy = self
            .cache_config()
            .cache_policy()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("none"));

        let repair_mode = self
            .repair_mode()
            .repair_mode()
            .map(|v| v.as_str().to_string())
            .unwrap_or_else(|| String::from("none"));

        let (
            inconsistency_total,
            inconsistency_only_in_cache,
            inconsistency_only_in_disk,
            inconsistency_value_mismatch,
        ) = match self.consistency().verify_consistency() {
            Ok(report) => (
                report.total_issues().to_string(),
                report.only_in_cache.to_string(),
                report.only_in_disk.to_string(),
                report.value_mismatches.to_string(),
            ),
            Err(err) if err.kind() == io::ErrorKind::Unsupported => (
                String::from("none"),
                String::from("none"),
                String::from("none"),
                String::from("none"),
            ),
            Err(err) => return Err(err),
        };

        let (
            last_repair_target,
            last_repair_total,
            last_repair_only_in_cache,
            last_repair_only_in_disk,
            last_repair_value_mismatch,
        ) = match self.repair().last_repair_summary() {
            Some(summary) => {
                let target = match summary.target {
                    RepairTarget::Disk => "disk",
                    RepairTarget::Cache => "cache",
                };
                (
                    target.to_string(),
                    summary.total_repairs().to_string(),
                    summary.repaired_only_in_cache.to_string(),
                    summary.repaired_only_in_disk.to_string(),
                    summary.repaired_value_mismatches.to_string(),
                )
            }
            None => (
                String::from("none"),
                String::from("none"),
                String::from("none"),
                String::from("none"),
                String::from("none"),
            ),
        };

        Ok(StatusSnapshot {
            engine: self.kv().engine_name(),
            len: self.kv().len(),
            stats,
            wal_path,
            wal_bytes,
            snapshot_path,
            snapshot_bytes,
            snapshot_mtime_unix,
            cache_policy,
            cache_max_keys,
            cache_current_keys,
            repair_mode,
            inconsistency_total,
            inconsistency_only_in_cache,
            inconsistency_only_in_disk,
            inconsistency_value_mismatch,
            last_repair_target,
            last_repair_total,
            last_repair_only_in_cache,
            last_repair_only_in_disk,
            last_repair_value_mismatch,
        })
    }

    pub fn cache_max_keys_string(&mut self) -> String {
        self.cache_limits()
            .cache_max_keys()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("unsupported"))
    }

    pub fn cache_policy_string(&mut self) -> String {
        self.cache_config()
            .cache_policy()
            .map(|v| v.to_string())
            .unwrap_or_else(|| String::from("unsupported"))
    }
}

pub trait Command {
    fn name(&self) -> &'static str;
    fn usage(&self) -> &'static str;
    fn description(&self) -> &'static str;

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput>;
}

pub struct CommandFactory {
    pub create: fn() -> Box<dyn Command>,
}

inventory::collect!(CommandFactory);

pub trait RegistrableCommand: Command + Default + 'static {
    fn create_boxed() -> Box<dyn Command> {
        Box::new(Self::default())
    }
}

impl<T> RegistrableCommand for T where T: Command + Default + 'static {}

#[macro_export]
macro_rules! submit_command {
    ($command_ty:ty) => {
        inventory::submit! {
            $crate::commands::base::CommandFactory {
                create: <$command_ty as $crate::commands::base::RegistrableCommand>::create_boxed
            }
        }
    };
}

pub struct CommandRegistry {
    commands: Vec<Box<dyn Command>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let commands = inventory::iter::<CommandFactory>
            .into_iter()
            .map(|factory| (factory.create)())
            .collect();

        Self { commands }
    }

    pub fn execute_line(
        &self,
        store: &mut dyn StorageEngine,
        line: &str,
    ) -> io::Result<CommandOutput> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(CommandOutput::none());
        }

        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or_default().to_ascii_lowercase();
        let args = parts.next().unwrap_or_default().trim_start();

        if name == "help" {
            return Ok(CommandOutput::message(self.help_text()));
        }

        if name == "exit" || name == "quit" {
            return Ok(CommandOutput::with_signal(
                Some(String::from("bye")),
                CommandSignal::Exit,
            ));
        }

        let mut ctx = CommandContext { store };
        if let Some(command) = self.find_command(&name) {
            return command.execute(&mut ctx, args);
        }

        Ok(CommandOutput::message(format!("unknown command: {name}")))
    }

    pub fn help_text(&self) -> String {
        let mut lines = vec![String::from("commands:")];

        let mut rows: Vec<_> = self
            .commands
            .iter()
            .map(|cmd| (cmd.name(), cmd.usage(), cmd.description()))
            .collect();

        rows.sort_by(|a, b| a.0.cmp(b.0));

        for (_, usage, description) in rows {
            lines.push(format!("  {usage}  - {description}"));
        }

        lines.push(String::from("  help  - show this help"));
        lines.push(String::from("  exit | quit  - exit cli"));

        lines.join("\n")
    }

    fn find_command(&self, name: &str) -> Option<&dyn Command> {
        self.commands
            .iter()
            .find(|cmd| cmd.name() == name || cmd.aliases().iter().any(|alias| *alias == name))
            .map(|cmd| cmd.as_ref())
    }
}
