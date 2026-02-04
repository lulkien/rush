use std::io::{Write, stderr, stdout};

use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;

use crate::plugin;

use super::BuiltinCommand;

static BUILTIN_NAME: &str = "plugin-help";
static HELP_STRING: &str = "plugin-help plugin_name";
static DESC_STRING: &str = "plugin-help is a shell built-in";

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn help(&self) -> &str {
        HELP_STRING
    }

    fn desc(&self) -> &str {
        DESC_STRING
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn exec(&self, mut args: RVec<RString>) -> rush_interface::ExecResult {
        if args.is_empty() || args.len() > 1 {
            return ExecResult::new(
                1,
                &format!(
                    "{}: expected 1 argument, found {}",
                    BUILTIN_NAME,
                    args.len()
                ),
            );
        }

        let plugin = args.remove(0);

        if let Ok(builtins_reg) = super::builtins_registry()
            && builtins_reg.contains(&plugin)
        {
            let output = builtins_reg
                .get_command(&plugin)
                .expect("Cannot get built-in command")
                .help()
                .to_owned();

            writeln!(stderr(), "{}", output).expect("Failed to write to stderr");

            ExecResult::default()
        } else {
            match plugin::get_plugin(&plugin) {
                Ok(plugin) => {
                    let output = plugin.help()().into_string();
                    writeln!(stdout(), "{output}").expect("Failed to write to stdout");
                    ExecResult::default()
                }
                Err(e) => ExecResult::new(101, &format!("{}: {e}", BUILTIN_NAME)),
            }
        }
    }
}
