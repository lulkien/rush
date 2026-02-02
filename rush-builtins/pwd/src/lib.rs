use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;
use std::{
    env,
    io::{Write, stdout},
    sync::OnceLock,
};

static COMMAND_INFO: OnceLock<CommandInfo> = OnceLock::new();

fn get_plugin_info() -> &'static CommandInfo {
    COMMAND_INFO.get_or_init(|| CommandInfo {
        name: env!("CARGO_PKG_NAME").into(),
        description: env!("CARGO_PKG_DESCRIPTION").into(),
        version: env!("CARGO_PKG_VERSION").into(),
        usage: "pwd".into(),
    })
}

#[info]
pub fn info() -> CommandInfo {
    get_plugin_info().clone()
}

#[usage]
pub fn usage() -> RString {
    get_plugin_info().usage.clone()
}

#[version]
pub fn version() -> RString {
    get_plugin_info().version.clone()
}

#[exec]
pub fn exec(_args: RVec<RString>) -> ExecResult {
    match env::current_dir() {
        Ok(path) => match stdout().write_all(format!("{}\n", path.to_string_lossy()).as_bytes()) {
            Ok(_) => {
                stdout().flush().unwrap();
                ExecResult::default()
            }
            Err(e) => ExecResult::new(1, &format!("{}", e)),
        },
        Err(e) => ExecResult::new(1, &e.to_string()),
    }
}

#[load]
pub fn load() {
    get_plugin_info();
}
