use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;
use std::{process, sync::OnceLock};

static COMMAND_INFO: OnceLock<CommandInfo> = OnceLock::new();

fn get_plugin_info() -> &'static CommandInfo {
    COMMAND_INFO.get_or_init(|| CommandInfo {
        name: "exit".into(),
        description: "Exit from current shell".into(),
        usage: "exit".into(),
        version: "0.1.0".into(),
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
    process::exit(0);
}

#[load]
pub fn load() {
    get_plugin_info();
}
