use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;
use std::env;

#[plugin_name]
pub fn plugin_name() -> RString {
    env!("CARGO_PKG_NAME").into()
}

#[plugin_desc]
pub fn plugin_desc() -> RString {
    env!("CARGO_PKG_DESCRIPTION").into()
}

#[plugin_version]
pub fn plugin_version() -> RString {
    env!("CARGO_PKG_VERSION").into()
}

#[plugin_help]
pub fn plugin_help() -> RString {
    "pwd".into()
}

#[execute]
pub fn execute(_args: RVec<RString>) -> ExecResult {
    match env::current_dir() {
        Ok(path) => ExecResult::new(0, &path.to_string_lossy()),
        Err(e) => ExecResult::new(1, &e.to_string()),
    }
}

#[load]
pub fn load() {}
