use std::io::{self, Write};

use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;

pub extern "C" fn prompt(_: RVec<RString>) -> ExecResult {
    print!("$ ");
    match io::stdout().flush() {
        Ok(_) => ExecResult::default(),
        Err(e) => {
            unreachable!("{e}")
        }
    }
}
