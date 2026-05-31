use abi_stable::std_types::{RString, RVec};
use rush_interface::CommandResult;

use super::BuiltinCommand;

static BUILTIN_NAME: &str = "true";
static DESC_STRING: &str = "Return a successful result.\ntrue is a shell built-in";

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

    fn execute(&self, args: RVec<RString>) -> CommandResult {
        if let Some(arg) = args.iter().next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    self.print_help();
                    return CommandResult::ok();
                }
                "-v" | "--version" => {
                    self.print_version();
                    return CommandResult::ok();
                }
                _ => {
                    return CommandResult::new(
                        255,
                        &format!("{BUILTIN_NAME}: unexpected argument '{arg}'"),
                    );
                }
            }
        }
        // No args → success, no output.
        CommandResult::ok()
    }
}
