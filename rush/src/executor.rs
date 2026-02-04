use std::{
    io::{Write, stderr},
    str::FromStr,
};

use abi_stable::std_types::{RString, RVec};
use log::debug;
use rush_interface::ExecResult;

use crate::{plugin::get_plugin, shell_builtins};

pub fn execute_user_input(input: &str) {
    if input.is_empty() {
        return;
    }

    let mut args: RVec<RString> = input
        .split_whitespace()
        .filter_map(|s| RString::from_str(s).ok())
        .collect();

    let cmd = args.remove(0);

    let status = if let Ok(builtins_reg) = shell_builtins::builtins_registry()
        && builtins_reg.contains(cmd.as_str())
    {
        builtins_reg.execute(&cmd, args)
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

pub fn init_module() -> anyhow::Result<()> {
    Ok(())
}
