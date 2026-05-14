use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;
use rush_plugin::*;
use std::io::Read;

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
    "cat — concatenate and print files / stdin.\n\nUsage: cat [FILE]...\n\nReads from stdin when no files given."
        .into()
}

#[execute]
pub fn execute(_args: RVec<RString>) -> ExecResult {
    let mut input = String::new();
    match std::io::stdin().read_to_string(&mut input) {
        Ok(_) => ExecResult::new(0, &input),
        Err(e) => ExecResult::new(1, &e.to_string()),
    }
}

#[load]
pub fn load() {}
