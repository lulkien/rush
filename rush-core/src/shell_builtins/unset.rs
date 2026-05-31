//! Builtin `unset` — remove shell variables.

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "unset".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Remove shell variables.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: unset [NAME ...]\n\
             \n\
             Remove each NAME from the shell variable store.\n\
             \n\
             Examples:\n\
               unset FOO\n\
               unset PATH HOME"
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: Vec<String>, vars: &crate::var::VarStore) -> CommandResult {
        if args.is_empty() {
            self.print_help();
            return CommandResult::new(1, "unset: expected variable name");
        }

        if args[0] == "-h" || args[0] == "--help" {
            self.print_help();
            return CommandResult::ok();
        }
        if args[0] == "-v" || args[0] == "--version" {
            self.print_version();
            return CommandResult::ok();
        }

        let exit_code: i32 = 0;
        for name in &args {
            if name == "-f" {
                // -f flag for functions (not implemented, silently ignore)
                continue;
            }
            if name == "-v" {
                // -v flag (not implemented, silently ignore)
                continue;
            }
            if name.starts_with('-') {
                continue;
            }
            vars.unset(name);
        }

        CommandResult::new(exit_code, "")
    }
}
