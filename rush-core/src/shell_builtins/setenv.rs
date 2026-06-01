//! Builtin `setenv` — set a shell variable.
//!
//! Usage: setenv VAR value...

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "setenv".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Set a shell variable.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: setenv VAR value...\n\
             \n\
             Set VAR to value (remaining arguments joined with space).\n\
             \n\
             Examples:\n\
               setenv FOO bar\n\
               setenv PATH /usr/bin:/bin"
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: Vec<String>, vars: &crate::var::VarStore) -> CommandResult {
        if args.is_empty() {
            self.print_help();
            return CommandResult::new(1, "");
        }
        if args[0] == "--help" || args[0] == "-h" {
            self.print_help();
            return CommandResult::ok();
        }

        let name = &args[0];
        let value = if args.len() > 1 {
            args[1..].join(" ")
        } else {
            String::new()
        };

        vars.set(name, vec![value]);
        CommandResult::ok()
    }
}
