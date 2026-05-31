use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;

use super::{BuiltinCommand, shared::EXIT_FAILURE};

static BUILTIN_NAME: &str = "false";
static DESC_STRING: &str = "Return an unsuccessful result.\nfalse is a shell built-in";

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn plugin_name(&self) -> RString {
        BUILTIN_NAME.into()
    }

    fn print_desc(&self) {
        eprintln!("{DESC_STRING}");
    }

    fn print_help(&self) {
        eprintln!(
            "{DESC_STRING}\n\nUsage: {name} [-h | -v]\n\nOptions:\n  -h, --help    Print this help message\n  -v, --version Print the version",
            name = BUILTIN_NAME,
        );
    }

    fn print_version(&self) {
        eprintln!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: RVec<RString>) -> ExecResult {
        if let Some(arg) = args.iter().next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    self.print_help();
                    return ExecResult::ok();
                }
                "-v" | "--version" => {
                    self.print_version();
                    return ExecResult::ok();
                }
                _ => {
                    return ExecResult::new(
                        255,
                        &format!("{BUILTIN_NAME}: unexpected argument '{arg}'"),
                    );
                }
            }
        }
        // No args → failure, no output.
        ExecResult::new(EXIT_FAILURE, "")
    }
}
