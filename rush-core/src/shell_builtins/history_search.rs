//! Builtin `history-search` — interactive fuzzy history search via skim TUI.

use abi_stable::std_types::{RString, RVec};
use rush_interface::CommandResult;

use super::BuiltinCommand;

static BUILTIN_NAME: &str = "history-search";
static DESC_STRING: &str = "Interactive fuzzy history search (Ctrl+R alternative).";

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> RString {
        BUILTIN_NAME.into()
    }

    fn print_desc(&self) {
        eprintln!("{DESC_STRING}");
    }

    fn print_help(&self) {
        eprintln!(
            "{DESC_STRING}\n\nUsage: {name}\n\nOpens an interactive TUI for searching command history.\nUse Ctrl+R in the rusher shell for the same effect.\n\n  history-search    # launch history picker",
            name = BUILTIN_NAME,
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, _args: RVec<RString>) -> CommandResult {
        // This builtin is special — the REPL loop detects it and handles
        // it by calling the InputHandler's history_search method.
        // When reached via the normal executor path, just print a message.
        CommandResult::new(0, "history-search: use Ctrl+R in interactive mode")
    }
}
