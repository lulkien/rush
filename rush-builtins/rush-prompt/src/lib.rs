use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;
use std::{env, io::{self, Write}, sync::OnceLock};

static COMMAND_INFO: OnceLock<CommandInfo> = OnceLock::new();

fn get_plugin_info() -> &'static CommandInfo {
    COMMAND_INFO.get_or_init(|| CommandInfo {
        name: "rush_prompt".into(),
        description: "Print prompt".into(),
        usage: "rush_prompt".into(),
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
    let user = env::var("USER").unwrap_or_default();
    let workdir = env::current_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    print!("{} {} $> ", user, workdir);
    io::stdout().flush().unwrap();

    ExecResult::default()
}

#[load]
pub fn load() {
    get_plugin_info();
}
