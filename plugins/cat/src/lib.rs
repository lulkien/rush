use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;
use rush_plugin::*;

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
    "T.B.D".into()
}

#[execute]
pub fn execute(_args: RVec<RString>, last_result: ExecResult) -> ExecResult {
    ExecResult::new(last_result.code, &last_result.message)
}

#[load]
pub fn load() {}
