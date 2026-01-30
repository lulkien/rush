use std::process;
use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;

pub extern "C" fn exit(_: RVec<RString>) -> ExecResult {
    process::exit(0);
}
