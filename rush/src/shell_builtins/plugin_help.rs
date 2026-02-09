use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;

use crate::plugin;

use super::{
    BuiltinCommand,
    shared::{INVALID_ARGS, NOT_A_PLUGIN, PLUGIN_NOT_FOUND},
};

static BUILTIN_NAME: &str = "plugin-help";
static DESC_STRING: &str = "Print a plugin's help.\nplugin-help is a shell built-in";

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn print_help(&self) {
        let usage = format!("Usage: {} [-h | -v | <plugin-name>]", BUILTIN_NAME);
        let options = [
            ("-h, --help", "Prints this help message"),
            ("-v, --version", "Prints the version"),
        ];

        let examples = [
            format!("{} -h", BUILTIN_NAME),
            format!("{} -v", BUILTIN_NAME),
            format!("{} echo", BUILTIN_NAME),
        ];

        let options_text = options
            .iter()
            .map(|(opt, desc)| format!("  {}: {}", opt, desc))
            .collect::<Vec<_>>()
            .join("\n");

        let examples_text = examples.join("\n");

        eprintln!(
            "{desc}\n\n{usage}\n\nOptions:\n{options}\n\nExamples:\n{examples}",
            desc = DESC_STRING,
            usage = usage,
            options = options_text,
            examples = examples_text,
        )
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: RVec<RString>) -> rush_interface::ExecResult {
        let [param] = args.as_slice() else {
            return ExecResult::new(
                INVALID_ARGS,
                &format!("{BUILTIN_NAME}: expected 1 argument, found {}", args.len()),
            );
        };

        match param.as_str() {
            "-h" | "--help" => self.print_help(),
            "-v" | "--version" => self.print_version(),
            _ => return self.handle_plugin_lookup(param),
        }

        ExecResult::ok()
    }
}

impl Command {
    fn handle_plugin_lookup(&self, param: &RString) -> ExecResult {
        plugin::get_plugin(param).map_or_else(
            |e| {
                let is_builtin = super::builtins_registry().is_ok_and(|reg| reg.contains(param));

                if is_builtin {
                    ExecResult::new(
                        NOT_A_PLUGIN,
                        &format!("{BUILTIN_NAME}: {param} is a shell builtin"),
                    )
                } else {
                    ExecResult::new(PLUGIN_NOT_FOUND, &format!("{BUILTIN_NAME}: {e}"))
                }
            },
            |plugin| {
                plugin.print_help()();
                ExecResult::ok()
            },
        )
    }
}
