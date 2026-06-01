//! Builtin `replace` — replace characters in a variable's value at a position.
//!
//! Usage: replace VAR pos len new_value

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "replace".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Replace a range of characters in a variable's value.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: replace VAR pos len new_value\n\
             \n\
             Replace characters at byte position `pos` for `len` bytes with `new_value`.\n\
             Prints the result (does not modify the variable).\n\
             \n\
             Example:\n\
               replace FOO 0 3 bar"
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: Vec<String>, vars: &crate::var::VarStore) -> CommandResult {
        if args.len() < 4 || args[0] == "--help" || args[0] == "-h" {
            self.print_help();
            return CommandResult::ok();
        }

        let value = vars.expand(&args[0]);
        let pos: usize = args[1].parse().unwrap_or(0);
        let len: usize = args[2].parse().unwrap_or(0);
        let replacement = &args[3];

        let pos = pos.min(value.len());
        let end = (pos + len).min(value.len());

        let mut result = String::with_capacity(value.len() - (end - pos) + replacement.len());
        result.push_str(&value[..pos]);
        result.push_str(replacement);
        result.push_str(&value[end..]);

        println!("{result}");
        CommandResult::ok()
    }
}
