//! Builtin `cd` — change working directory.

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "cd".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Change the current working directory.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: cd [directory]\n\
             \n\
             Change the shell working directory.\n\
             If no directory is given, changes to $HOME.\n\
             \n\
             Examples:\n\
               cd /tmp\n\
               cd          # go to $HOME\n\
               cd -        # go to previous directory (not yet implemented)\n"
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: Vec<String>, _vars: &crate::var::VarStore) -> CommandResult {
        let dir = if args.is_empty() {
            std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
        } else if args[0] == "-h" || args[0] == "--help" {
            self.print_help();
            return CommandResult::ok();
        } else if args[0] == "-v" || args[0] == "--version" {
            self.print_version();
            return CommandResult::ok();
        } else {
            args[0].clone()
        };

        if args.len() > 1 {
            return CommandResult::new(
                1,
                &format!("cd: too many arguments (expected 0-1, got {})", args.len()),
            );
        }

        match std::env::set_current_dir(&dir) {
            Ok(()) => CommandResult::ok(),
            Err(e) => CommandResult::new(1, &format!("cd: {}: {}", dir, e)),
        }
    }
}
