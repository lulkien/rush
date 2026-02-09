use std::process;

use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;

use super::{BuiltinCommand, shared::INVALID_ARGS};

static BUILTIN_NAME: &str = "exit";
static DESC_STRING: &str = "Exit from current shell with code.\nexit is a shell built-in";

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn print_help(&self) {
        let usage = format!("Usage: {} [-h | -v | <plugin-name>]", BUILTIN_NAME);
        let options = [
            ("-h, --help", "Prints this help message"),
            ("-v, --version", "Prints the version"),
        ];

        let examples = [
            format!("{} -h", BUILTIN_NAME),
            format!("{} -v", BUILTIN_NAME),
            format!("{} 127", BUILTIN_NAME),
        ];

        let options_text = options
            .iter()
            .map(|(opt, desc)| format!("  {}: {}", opt, desc))
            .collect::<Vec<_>>()
            .join("\n");

        let examples_text = examples.join("\n");

        eprintln!(
            "{desc}\n\n{usage}\n\nOptions:\n{options}\n\nExamples:\n{examples}",
            desc = DESC_STRING,
            usage = usage,
            options = options_text,
            examples = examples_text,
        )
    }

    fn print_version(&self) {
        println!("{}", env!("CARGO_PKG_VERSION"));
    }

    fn execute(&self, args: RVec<RString>) -> rush_interface::ExecResult {
        match args.as_slice() {
            [] => process::exit(0),

            [param] => match param.as_str() {
                "-h" => {
                    self.print_help();
                    ExecResult::ok()
                }
                "-v" => {
                    self.print_version();
                    ExecResult::ok()
                }
                _ => param
                    .parse::<u8>()
                    .map(|val| process::exit(val as i32))
                    .unwrap_or_else(|_| {
                        ExecResult::new(255, &format!("{BUILTIN_NAME}: expected u8, found {param}"))
                    }),
            },

            _ => ExecResult::new(
                INVALID_ARGS,
                &format!(
                    "{BUILTIN_NAME}: expected [0-1] argument, found {}",
                    args.len()
                ),
            ),
        }
    }
}
