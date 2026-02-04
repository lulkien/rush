use std::io::{Write, stderr, stdout};

use rush_interface::ExecResult;

use crate::plugin;

use super::BuiltinCommand;

static HELP_STRING: &str = "plugin-version plugin_name";
static DESC_STRING: &str = "plugin-version is a shell built-in";

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

    fn exec(&self, plugin: &str) -> rush_interface::ExecResult {
        if let Ok(builtins_reg) = super::builtins_registry()
            && builtins_reg.contains(plugin)
        {
            let output = builtins_reg
                .get_command(plugin)
                .expect("Cannot get built-in command")
                .version()
                .to_owned();

            writeln!(stderr(), "{}", output).expect("Failed to write to stderr");

            ExecResult::default()
        } else {
            match plugin::get_plugin(plugin) {
                Ok(plugin) => {
                    let output = plugin.version()().into_string();
                    writeln!(stdout(), "{output}").expect("Failed to write to stdout");
                    ExecResult::default()
                }
                Err(e) => ExecResult::new(101, &format!("{e}")),
            }
        }
    }
}
