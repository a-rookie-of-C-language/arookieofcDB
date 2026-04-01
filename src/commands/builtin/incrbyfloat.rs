use std::io;

use crate::commands::base::{Command, CommandContext, CommandOutput};
use crate::key_codec::KeyEncoding;
use crate::value_codec::StringEncoding;

use super::utils::{invalid_input, parse_key};

#[derive(Default)]
struct IncrByFloatCommand;

impl Command for IncrByFloatCommand {
    fn name(&self) -> &'static str { "incrbyfloat" }
    fn usage(&self) -> &'static str { "incrbyfloat <key> <increment>" }
    fn description(&self) -> &'static str { "increment numeric value by a floating-point number" }

    fn execute(&self, ctx: &mut CommandContext<'_>, args: &str) -> io::Result<CommandOutput> {
        let mut parts = args.split_whitespace();
        let key = parse_key(parts.next(), "missing key for incrbyfloat")?;
        let delta_raw = parts.next().ok_or_else(|| invalid_input("missing increment for incrbyfloat"))?;
        let delta = delta_raw.parse::<f64>().map_err(|_| invalid_input("invalid float increment"))?;

        if !delta.is_finite() {
            return Err(invalid_input("float increment must be finite"));
        }

        let current = current_f64(ctx, &key)?;
        let next = current + delta;
        if !next.is_finite() {
            return Err(invalid_input("float overflow"));
        }

        ctx.kv().set(key, StringEncoding::Float(next).encode())?;
        Ok(CommandOutput::message(next.to_string()))
    }
}

fn current_f64(ctx: &mut CommandContext<'_>, key: &KeyEncoding) -> io::Result<f64> {
    let Some(raw) = ctx.kv().get(key) else { return Ok(0.0); };

    match StringEncoding::decode(raw) {
        StringEncoding::Int(v) => Ok(v as f64),
        StringEncoding::Float(v) => Ok(v),
        StringEncoding::Raw(bytes) => {
            let text = String::from_utf8_lossy(&bytes);
            let parsed = text.parse::<f64>().map_err(|_| invalid_input("value is not a float"))?;
            if !parsed.is_finite() {
                return Err(invalid_input("value is not a finite float"));
            }
            Ok(parsed)
        }
    }
}

crate::submit_command!(IncrByFloatCommand);
