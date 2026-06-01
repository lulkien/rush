//! Builtin `strip` — prefix/suffix removal on a variable's value.
//!
//! Usage: strip VAR --start [--long] PATTERN
//!        strip VAR --end   [--long] PATTERN

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "strip".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Strip a prefix or suffix from a variable's value.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: strip VAR --start [--long] PATTERN\n\
                   strip VAR --end   [--long] PATTERN\n\
             \n\
             --start     Remove a prefix matching PATTERN (glob).\n\
             --end       Remove a suffix matching PATTERN (glob).\n\
             --long      Greedy: remove the longest match instead of shortest.\n\
             \n\
             Examples:\n\
               strip PATH --start '*/'\n\
               strip NAME --end --long '.*'"
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: Vec<String>, vars: &crate::var::VarStore) -> CommandResult {
        if args.len() < 3 || args[0] == "--help" || args[0] == "-h" {
            self.print_help();
            return CommandResult::ok();
        }

        let var_name = &args[0];
        let value = vars.expand(var_name);

        // Parse flags
        let mut direction: Option<&str> = None;
        let mut long = false;
        let mut pattern_idx = 1;

        while pattern_idx < args.len() {
            match args[pattern_idx].as_str() {
                "--start" => {
                    direction = Some("start");
                    pattern_idx += 1;
                }
                "--end" => {
                    direction = Some("end");
                    pattern_idx += 1;
                }
                "--long" => {
                    long = true;
                    pattern_idx += 1;
                }
                _ => break,
            }
        }

        let Some(dir) = direction else {
            self.print_help();
            return CommandResult::new(1, "");
        };

        if pattern_idx >= args.len() {
            self.print_help();
            return CommandResult::new(1, "");
        }

        let pattern = &args[pattern_idx];
        let result = match (dir, long) {
            ("start", false) => crate::glob::remove_shortest_prefix(&value, pattern),
            ("start", true) => crate::glob::remove_longest_prefix(&value, pattern),
            ("end", false) => crate::glob::remove_shortest_suffix(&value, pattern),
            ("end", true) => crate::glob::remove_longest_suffix(&value, pattern),
            _ => value,
        };

        println!("{result}");
        CommandResult::ok()
    }
}
