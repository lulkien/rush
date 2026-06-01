//! Builtin `print` — write arguments to stdout.
//!
//! Usage: print arg...

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "print".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Write arguments to stdout, separated by spaces.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: print arg...\n\
             \n\
             Join all arguments with a space and print to stdout.\n\
             \n\
             Example:\n\
               print hello world"
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: Vec<String>, _vars: &crate::var::VarStore) -> CommandResult {
        if !args.is_empty() && (args[0] == "--help" || args[0] == "-h") {
            self.print_help();
            return CommandResult::ok();
        }

        println!("{}", args.join(" "));
        CommandResult::ok()
    }
}
