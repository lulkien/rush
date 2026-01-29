use abi_stable::std_types::RVec;

use crate::{init::init_shell, plugin::{get_plugin, start_plugin_subsystem}};

mod init;
mod plugin;

fn main() {
    if let Err(e) = start_shell() {
        eprintln!("{e}");
    }
}

fn start_shell() -> anyhow::Result<()> {
    init_shell()?;
    start_plugin_subsystem()?;

    get_plugin("pwd")?.exec()(RVec::new());

    Ok(())
}
