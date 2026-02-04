#![allow(unused)]

use std::{
    io::{Write, stderr, stdout},
    process,
    str::FromStr,
};

use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;

use crate::plugin;

use super::BuiltinCommand;

static BUILTIN_NAME: &str = "exit";
static HELP_STRING: &str = "exit [exit_code]";
static DESC_STRING: &str = "exit is a shell built-in";

pub(super) struct Command;

impl BuiltinCommand for Command {
    fn help(&self) -> &str {
        HELP_STRING
    }

    fn desc(&self) -> &str {
        DESC_STRING
    }

    fn version(&self) -> &str {
        env!("CARGO_PKG_VERSION")
    }

    fn exec(&self, mut args: RVec<RString>) -> rush_interface::ExecResult {
        let exit_code: u8 = match args.len() {
            0 => 0,
            1 => match args[0].parse() {
                Ok(value) => value,
                Err(_) => {
                    return ExecResult::new(
                        255,
                        &format!("{}: expected u8, found {}", BUILTIN_NAME, args[0]),
                    );
                }
            },
            _ => {
                return ExecResult::new(
                    1,
                    &format!(
                        "{}: expected [0-1] arguments, found {}",
                        BUILTIN_NAME,
                        args.len()
                    ),
                );
            }
        };

        process::exit(exit_code as i32);
    }
}
