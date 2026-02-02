use std::{
    io::{Write, stderr, stdout},
    str::FromStr,
    sync::OnceLock,
};

use abi_stable::std_types::{RString, RVec};
use log::{debug, warn};
use rush_interface::ExecResult;

use crate::plugin::{self, get_plugin};

static SHELL_CMD: OnceLock<Vec<&'static str>> = OnceLock::new();

fn get_shell_cmd() -> &'static Vec<&'static str> {
    SHELL_CMD.get_or_init(|| {
        vec![
            "plugin_info",
            "plugin_reload",
            "plugin_usage",
            "plugin_version",
        ]
    })
}

pub fn execute_user_input(input: &str) {
    if input.is_empty() {
        return;
    }

    let mut args: RVec<RString> = input
        .split_whitespace()
        .filter_map(|s| RString::from_str(s).ok())
        .collect();

    let cmd = args.remove(0);

    let status = if get_shell_cmd().contains(&cmd.as_str()) {
        if args.is_empty() {
            ExecResult::new(100, "No argument")
        } else {
            // Consume next argument
            let plugin_name = args.remove(0);

            if !args.is_empty() {
                warn!("{}: Too many arguments", cmd);
            }
            execute_shell_command(&cmd, plugin_name.as_str())
        }
    } else {
        execute_command(&cmd, args)
    };

    debug!("{:?}", status);

    if status.code.ne(&0) {
        let _ = stderr().write_all(format!("{}\n", status.message).as_bytes());
    }
}

pub fn execute_command(cmd: &str, argv: RVec<RString>) -> ExecResult {
    match get_plugin(cmd) {
        Ok(plugin) => plugin.exec()(argv),
        Err(e) => ExecResult::new(101, &format!("{e}")),
    }
}

fn execute_shell_command(shell_cmd: &str, plugin_name: &str) -> ExecResult {
    let plugin = match get_plugin(plugin_name) {
        Ok(plugin) => plugin,
        Err(e) => return ExecResult::new(101, &format!("{e}")),
    };

    // FIXME: no need to load plugin first in case of plugin_reload
    let output = match shell_cmd {
        "plugin_version" => plugin.version()(),
        "plugin_usage" => plugin.usage()(),
        "plugin_reload" => {
            drop(plugin);
            plugin::reload_plugin(plugin_name)
                .map(|_| format!("{}: Plugin reloaded", plugin_name).into())
                .unwrap_or_else(|err_value| format!("{}", err_value).into())
        }
        _ => return ExecResult::new(102, &format!("{}: Unimplemented shell command", shell_cmd)),
    };

    stdout()
        .write_all(format!("{}\n", output).as_bytes())
        .unwrap();

    ExecResult::default()
}

pub fn init_module() -> anyhow::Result<()> {
    get_shell_cmd();

    Ok(())
}
