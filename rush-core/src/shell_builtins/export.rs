//! Builtin `export` — mark variables for export to child processes.

use rush_interface::CommandResult;

use super::BuiltinCommand;

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> String {
        "export".to_string()
    }

    fn print_desc(&self) {
        eprintln!("Mark variables for export to child processes.");
    }

    fn print_help(&self) {
        eprintln!(
            "Usage: export [NAME[=value] ...]\n\
             \n\
             Mark each NAME for export to child processes.\n\
             If =value is given, also set the variable.\n\
             With no arguments, list all exported variables.\n\
             \n\
             Examples:\n\
               export PATH\n\
               export FOO=bar\n\
               export"
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: Vec<String>, _vars: &crate::var::VarStore) -> CommandResult {
        // No args: list exported vars (handled externally for now).
        // For now, just a no-op compatibility layer — all vars are exported
        // by default unless unset is used to remove them.
        if args.is_empty() {
            return CommandResult::ok();
        }

        if args[0] == "-h" || args[0] == "--help" {
            self.print_help();
            return CommandResult::ok();
        }
        if args[0] == "-v" || args[0] == "--version" {
            self.print_version();
            return CommandResult::ok();
        }

        // export is handled by the shell's assignment mechanism —
        // `export FOO=bar` is parsed as `FOO=bar export` which sets FOO
        // in the preprocess step. The "export" command itself is a no-op.
        // Variables set via export are already in VarStore and exported
        // by default.
        CommandResult::ok()
    }
}
