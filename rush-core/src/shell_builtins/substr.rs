//! Builtin `substr` — substring of a variable.
//!
//! Usage: substr VAR offset [length]

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "substr".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Print a substring of a variable's value.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: substr VAR offset [length]\n\
             \n\
             Print characters from offset to end (or offset+length if given).\n\
             \n\
             Examples:\n\
               substr HOME 0 5\n\
               substr PATH 10"
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: Vec<String>, vars: &crate::var::VarStore) -> CommandResult {
        if args.len() < 2 || args[0] == "--help" || args[0] == "-h" {
            self.print_help();
            return CommandResult::ok();
        }

        let value = vars.expand(&args[0]);
        let offset: usize = args[1].parse().unwrap_or(0);
        let len: Option<usize> = args.get(2).and_then(|s| s.parse().ok());

        let chars: Vec<char> = value.chars().collect();
        let start = offset.min(chars.len());
        let end = match len {
            Some(l) => (start + l).min(chars.len()),
            None => chars.len(),
        };

        println!("{}", chars[start..end].iter().collect::<String>());
        CommandResult::ok()
    }
}
