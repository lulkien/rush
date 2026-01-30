use abi_stable::std_types::RVec;

mod builtins;
mod commands;
mod executor;
mod init;
mod input;
mod plugin;

fn main() {
    if let Err(e) = start_shell() {
        eprintln!("{e}");
    }
}

fn start_shell() -> anyhow::Result<()> {
    init::init_shell()?;
    plugin::start_plugin_subsystem()?;

    loop {
        commands::lock_shell_commands_read()?.execute_command("rush_prompt", RVec::new());

        let input = input::get_user_input()?;
        executor::execute_user_input(&input);
    }
}

