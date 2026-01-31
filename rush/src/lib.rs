use abi_stable::std_types::RVec;

mod builtins;
mod commands;
mod env;
mod executor;
mod init;
mod input;
mod plugin;

pub fn start_shell() -> anyhow::Result<()> {
    // Init init module
    init::init_module()?;

    // Init env module and add user paths
    env::init_module()?;
    env::add_rush_config_dirs(init::get_user_config_dir()?, true)?;
    env::add_rush_data_dirs(init::get_user_data_dir()?, true)?;

    // Init plugin module
    plugin::init_module()?;

    //
    enter_main_loop()?;

    unreachable!()
}

fn enter_main_loop() -> anyhow::Result<()> {
    // Enter main loop
    loop {
        commands::lock_shell_commands_read()?.execute_command("rush_prompt", RVec::new());

        let input = input::get_user_input()?;
        executor::execute_user_input(&input);
    }
}
