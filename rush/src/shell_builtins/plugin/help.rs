use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;

use crate::shell_builtins::{
    plugin::{BUILTIN_NAME, PluginLookUp, PluginSubCommand},
    shared::{INVALID_ARGS, NOT_A_PLUGIN, PLUGIN_NOT_FOUND},
};

const SUB_COMMAND: &str = "help";

pub(super) struct SubCommand;

impl PluginSubCommand for SubCommand {
    fn sub_command_help() {
        eprintln!("Usage: {} {} [--] [plugin-name]", BUILTIN_NAME, SUB_COMMAND);
        eprintln!();
        eprintln!("Displays help information for a specific plugin.");
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  plugin-name       The name of the plugin to print help.");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --help            Show this help message.");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  {} {} my_plugin", BUILTIN_NAME, SUB_COMMAND);
    }

    fn execute<'a>(args: impl Iterator<Item = &'a RString>) -> ExecResult {
        let mut args = args.peekable();

        if args.peek().is_none() {
            return ExecResult::new(
                INVALID_ARGS,
                &format!("{}-{}: missing argument", BUILTIN_NAME, SUB_COMMAND),
            );
        }

        let mut first_arg = args.next().unwrap().as_str();
        if first_arg == "--" {
            if args.peek().is_none() {
                return ExecResult::new(
                    INVALID_ARGS,
                    &format!("{}-{}: missing argument", BUILTIN_NAME, SUB_COMMAND),
                );
            }

            first_arg = args.next().unwrap().as_str();
        }

        if args.peek().is_some() {
            return ExecResult::new(
                INVALID_ARGS,
                &format!("{}-{}: too many argument", BUILTIN_NAME, SUB_COMMAND),
            );
        }

        match first_arg {
            "--help" => {
                SubCommand::sub_command_help();
                ExecResult::ok()
            }
            _ => match super::plugin_lookup(first_arg) {
                PluginLookUp::ShellBuiltin(name) => ExecResult::new(
                    NOT_A_PLUGIN,
                    &format!(
                        "{}-{}: {} is a shell builtin",
                        BUILTIN_NAME, SUB_COMMAND, first_arg
                    ),
                ),
                PluginLookUp::Plugin(plugin) => {
                    plugin.print_help()();
                    ExecResult::ok()
                }
                PluginLookUp::NotFound => ExecResult::new(
                    PLUGIN_NOT_FOUND,
                    &format!(
                        "{}-{}: {} plugin not found",
                        BUILTIN_NAME, SUB_COMMAND, first_arg
                    ),
                ),
            },
        }
    }
}
