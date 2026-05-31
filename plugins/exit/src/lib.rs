use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;

#[plugin_name]
pub fn plugin_name() -> RString {
    "exit".into()
}

#[plugin_desc]
pub fn plugin_desc() -> RString {
    "Exit from current shell with an optional status code.".into()
}

#[plugin_version]
pub fn plugin_version() -> RString {
    env!("CARGO_PKG_VERSION").into()
}

#[plugin_help]
pub fn plugin_help() -> RString {
    "Usage: exit [code]\n\
\n\
Exit the shell with an optional exit code (0-255).\n\
\n\
Options:\n\
  -h, --help     Print this help message\n\
  -v, --version  Print the version\n\
\n\
Examples:\n\
  exit\n\
  exit 127\n"
        .into()
}

#[execute]
pub fn execute(args: RVec<RString>) -> CommandResult {
    match args.as_slice() {
        [] => std::process::exit(0),

        [param] => match param.as_str() {
            "-h" | "--help" => CommandResult {
                code: 255,
                message: rush_internal_plugin_help(),
            },
            "-v" | "--version" => CommandResult {
                code: 255,
                message: rush_internal_plugin_version(),
            },
            _ => param
                .parse::<u8>()
                .map(|val| std::process::exit(val as i32))
                .unwrap_or_else(|_| {
                    CommandResult::new(255, &format!("exit: expected u8, found {param}"))
                }),
        },

        _ => CommandResult::new(
            2,
            &format!(
                "exit: expected [0-1] argument, found {}",
                args.len()
            ),
        ),
    }
}

#[load]
pub fn load() {}
