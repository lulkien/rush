use abi_stable::{
    StableAbi, declare_root_module_statics, library::RootModule, package_version_strings,
    sabi_types::VersionStrings, std_types::{RString, RVec},
};

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = CommandRef)))]
#[sabi(missing_field(panic))]
pub struct Command {
    pub load: extern "C" fn(),
    pub info: extern "C" fn() -> CommandInfo,
    pub version: extern "C" fn() -> RString,
    pub usage: extern "C" fn() -> RString,
    pub exec: extern "C" fn(RVec<RString>) -> ExecResult,
}

#[repr(C)]
#[derive(StableAbi, Debug, Clone)]
pub struct CommandInfo {
    pub name: RString,
    pub description: RString,
    pub usage: RString,
    pub version: RString,
}

#[repr(C)]
#[derive(StableAbi, Debug, Clone)]
pub struct ExecResult {
    pub status: u8,
    pub message: RString,
}

impl RootModule for CommandRef {
    declare_root_module_statics! {CommandRef}

    const BASE_NAME: &'static str = "rush_plugin";
    const NAME: &'static str = "rush_plugin";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}
