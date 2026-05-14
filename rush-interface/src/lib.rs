use abi_stable::{
    StableAbi, declare_root_module_statics,
    library::RootModule,
    package_version_strings,
    sabi_types::VersionStrings,
    std_types::{RString, RVec},
};

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = CommandRef)))]
#[sabi(missing_field(panic))]
pub struct Command {
    pub load: extern "C" fn(),
    pub plugin_name: extern "C" fn() -> RString,
    pub plugin_help: extern "C" fn() -> RString,
    pub plugin_desc: extern "C" fn() -> RString,
    pub plugin_version: extern "C" fn() -> RString,
    pub execute: extern "C" fn(RVec<RString>, ExecResult) -> ExecResult,
}

#[repr(C)]
#[derive(StableAbi, Debug, Clone, Default)]
pub struct ExecResult {
    pub code: u8,
    pub message: RString,
}

impl ExecResult {
    pub fn new(code: u8, message: &str) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn ok() -> Self {
        ExecResult::default()
    }
}

impl RootModule for CommandRef {
    declare_root_module_statics! {CommandRef}

    const BASE_NAME: &'static str = "rush_plugin";
    const NAME: &'static str = "rush_plugin";
    const VERSION_STRINGS: VersionStrings = package_version_strings!();
}
