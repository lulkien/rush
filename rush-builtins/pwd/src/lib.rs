use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;
use std::{env, sync::OnceLock};

static COMMAND_INFO: OnceLock<CommandInfo> = OnceLock::new();

fn get_plugin_info() -> &'static CommandInfo {
    COMMAND_INFO.get_or_init(|| CommandInfo {
        name: "pwd".into(),
        description: "Print working directory".into(),
        usage: "pwd".into(),
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
    match env::current_dir() {
        Ok(path) => {
            println!("{}", path.to_string_lossy());
            ExecResult {
                status: 0,
                message: "".into(),
            }
        }
        Err(e) => ExecResult {
            status: 1,
            message: e.to_string().into(),
        },
    }
}

#[load]
pub fn load() {
    get_plugin_info();
}
