//! Builtin `len` — string length of a variable.
//!
//! Usage: len VAR

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "len".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Print the length of a variable's value.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: len VAR\n\
             \n\
             Print the character length of VAR's value.\n\
             \n\
             Example:\n\
               len HOME"
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

        let value = vars.expand(&args[0]);
        println!("{}", value.chars().count());
        CommandResult::ok()
    }
}
