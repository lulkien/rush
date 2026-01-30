use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;
use std::io::{self, Write};
use std::process;

pub extern "C" fn prompt(_: RVec<RString>) -> ExecResult {
    print!("$ ");
    match io::stdout().flush() {
        Ok(_) => ExecResult::default(),
        Err(e) => {
            unreachable!("{e}")
        }
    }
}

pub extern "C" fn exit(_: RVec<RString>) -> ExecResult {
    process::exit(0);
}
