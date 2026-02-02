use std::time::Instant;

use abi_stable::std_types::RVec;
use log::info;

mod env;
mod executor;
mod init;
mod input;
mod plugin;

pub fn start_shell() -> anyhow::Result<()> {
    let start = Instant::now();

    // Init init module
    init::init_module()?;

    // Init env module and add user paths
    env::init_module()?;
    env::add_rush_data_dirs(init::get_user_data_dir()?, true)?;
    env::add_rush_config_dirs(init::get_user_config_dir()?, true)?;

    // Init plugin module
    plugin::init_module()?;

    executor::init_module()?;

    let elapsed = start.elapsed();
    info!("Shell initialization took: {} µs", elapsed.as_micros());

    //
    enter_main_loop()?;

    unreachable!()
}

fn enter_main_loop() -> anyhow::Result<()> {
    // Enter main loop
    loop {
        executor::execute_command("rush_prompt", RVec::new());

        let input = input::get_user_input()?;
        executor::execute_user_input(&input);
    }
}
