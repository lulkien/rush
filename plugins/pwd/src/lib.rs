use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;
use std::env;

#[plugin_name]
pub fn plugin_name() -> RString {
    env!("CARGO_PKG_NAME").into()
}

#[print_desc]
pub fn print_desc() {
    eprintln!("{}", env!("CARGO_PKG_DESCRIPTION"));
}

#[print_help]
pub fn print_help() {
    eprintln!("pwd");
}

#[print_version]
pub fn print_version() {
    eprintln!("{}", env!("CARGO_PKG_VERSION"));
}

#[execute]
pub fn execute(_args: RVec<RString>) -> ExecResult {
    match env::current_dir() {
        Ok(path) => {
            println!("{}", path.to_string_lossy());
            ExecResult::default()
        }
        Err(e) => ExecResult::new(1, &e.to_string()),
    }
}

#[load]
pub fn load() {}
