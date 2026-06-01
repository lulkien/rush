//! Builtin `getenv` — get a shell variable value.
//!
//! Usage: getenv VAR [--default=VALUE]

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "getenv".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Get the value of a shell variable.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: getenv VAR [--default=VALUE]\n\
             \n\
             Print the value of VAR. If VAR is unset or empty and --default\n\
             is given, print VALUE instead.\n\
             \n\
             Examples:\n\
               getenv HOME\n\
               getenv FOO --default=fallback"
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: Vec<String>, vars: &crate::var::VarStore) -> CommandResult {
        if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
            self.print_help();
            return CommandResult::ok();
        }

        let name = &args[0];
        let mut default: Option<String> = None;

        for arg in &args[1..] {
            if let Some(val) = arg.strip_prefix("--default=") {
                default = Some(val.to_string());
            }
        }

        let mut value = vars.expand(name);
        if value.is_empty()
            && let Some(d) = default {
                value = d;
            }

        println!("{value}");
        CommandResult::ok()
    }
}
