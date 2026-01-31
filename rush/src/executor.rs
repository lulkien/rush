use std::{
    io::{self, Write},
    str::FromStr,
};

use abi_stable::std_types::{RString, RVec};
use log::debug;
use rush_interface::ExecResult;

use crate::{commands::read_shell_commands, plugin::get_plugin};

pub struct ExecutorWrapper(extern "C" fn(RVec<RString>) -> ExecResult);

impl ExecutorWrapper {
    pub fn new(command_fn: extern "C" fn(RVec<RString>) -> ExecResult) -> Self {
        Self(command_fn)
    }

    pub fn exec(&self, args: RVec<RString>) -> ExecResult {
        (self.0)(args)
    }
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

    let status = match get_plugin(&cmd) {
        Ok(command) => command.exec()(args),
        Err(_e) => match read_shell_commands() {
            Ok(guard) => guard.execute_command(&cmd, args),
            Err(e) => ExecResult::new(101, &format!("{cmd}: {e}")),
        },
    };

    debug!("{:?}", status);
    eprintln!("{}", status.message);
    io::stderr().flush().unwrap();
}
