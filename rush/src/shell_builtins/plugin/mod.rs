#![allow(unused)]

use std::sync::Arc;

use abi_stable::std_types::{RString, RVec};
use rush_interface::{CommandRef, ExecResult};

use crate::plugin;

use super::{
    BuiltinCommand,
    shared::{INVALID_ARGS, NOT_A_PLUGIN, PLUGIN_NOT_FOUND},
};

mod description;
mod help;
mod version;

static BUILTIN_NAME: &str = "plugin";
static DESC_STRING: &str = "";

enum PluginLookUp {
    ShellBuiltin(String),
    Plugin(Arc<CommandRef>),
    NotFound,
}

trait PluginSubCommand {
    fn sub_command_help();
    fn execute<'a>(args: impl Iterator<Item = &'a RString>) -> ExecResult;
}

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn print_help(&self) {
        eprintln!("Usage: {} [sub-command] [options]", BUILTIN_NAME);
        eprintln!();
        eprintln!("Sub-commands:");
        eprintln!("  desc, description   Display the description of the plugin.");
        eprintln!("  help                Show help information for the plugin.");
        eprintln!("  version             Show the version of the plugin.");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -h, --help          Show this help message.");
        eprintln!("  -v, --version       Display the current version of the plugin.");
        eprintln!("\nNote: Replace [sub-command] with one of the available sub-commands.");
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, mut args: RVec<RString>) -> ExecResult {
        let mut args = args.iter().peekable();

        if let Some(first_arg) = args.next() {
            let first_arg = first_arg.as_str();

            match first_arg {
                "-h" | "--help" => {
                    if args.peek().is_some() {
                        ExecResult::new(
                            INVALID_ARGS,
                            &format!("{}: too many arguments", BUILTIN_NAME),
                        )
                    } else {
                        self.print_help();
                        ExecResult::ok()
                    }
                }
                "-v" | "--version" => {
                    if args.peek().is_some() {
                        ExecResult::new(
                            INVALID_ARGS,
                            &format!("{}: too many arguments", BUILTIN_NAME),
                        )
                    } else {
                        self.print_version();
                        ExecResult::ok()
                    }
                }
                "desc" | "description" => description::SubCommand::execute(args),
                "help" => help::SubCommand::execute(args),
                "version" => version::SubCommand::execute(args),
                _ => ExecResult::new(
                    PLUGIN_NOT_FOUND,
                    &format!("{}: {} sub-command not found", BUILTIN_NAME, first_arg),
                ),
            }
        } else {
            ExecResult::new(
                PLUGIN_NOT_FOUND,
                &format!("{}: missing sub-command", BUILTIN_NAME),
            )
        }
    }
}

fn plugin_lookup(name: &str) -> PluginLookUp {
    plugin::get_plugin(name).map_or_else(
        |e| {
            if super::builtins_registry().is_ok_and(|reg| reg.contains(name)) {
                PluginLookUp::ShellBuiltin(name.to_string())
            } else {
                PluginLookUp::NotFound
            }
        },
        PluginLookUp::Plugin,
    )
}
