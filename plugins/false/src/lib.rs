use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;

#[plugin_name]
pub fn plugin_name() -> RString {
    "false".into()
}

#[plugin_desc]
pub fn plugin_desc() -> RString {
    "Return an unsuccessful result.".into()
}

#[plugin_version]
pub fn plugin_version() -> RString {
    env!("CARGO_PKG_VERSION").into()
}

#[plugin_help]
pub fn plugin_help() -> RString {
    "Usage: false [-h | -v]\n\
\n\
Always returns failure (exit code 1).\n\
\n\
Options:\n\
  -h, --help     Print this help message\n\
  -v, --version  Print the version\n"
        .into()
}

#[execute]
pub fn execute(args: RVec<RString>) -> ExecResult {
    if let Some(arg) = args.iter().next() {
        match arg.as_str() {
            "-h" | "--help" => {
                return ExecResult {
                    code: 255,
                    message: rush_internal_plugin_help(),
                };
            }
            "-v" | "--version" => {
                return ExecResult {
                    code: 255,
                    message: rush_internal_plugin_version(),
                };
            }
            _ => {
                return ExecResult::new(
                    255,
                    &format!("false: unexpected argument '{arg}'"),
                );
            }
        }
    }
    ExecResult::new(1, "")
}

#[load]
pub fn load() {}
