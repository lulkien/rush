use std::{fs::File, time::Instant};

use abi_stable::std_types::RVec;
use log::{error, info};
use rustyline::error::ReadlineError;

mod env;
mod executor;
mod init;
mod input;
mod plugin;
mod shell_builtins;

pub fn start_shell() -> anyhow::Result<()> {
    let start = Instant::now();

    // Init init module
    init::init_module()?;

    // Init env module and add user paths
    env::init_module()?;
    env::add_rush_data_dirs(init::get_user_data_dir()?, true)?;
    env::add_rush_config_dirs(init::get_user_config_dir()?, true)?;

    shell_builtins::init_module()?;

    // Init plugin module
    plugin::init_module()?;

    // Init command executor module
    executor::init_module()?;

    // Init user input module
    input::init_module()?;

    let elapsed = start.elapsed();

    let elapsed_string = if elapsed.as_micros() < 1000 {
        format!("{} µs", elapsed.as_micros())
    } else if elapsed.as_millis() < 1000 {
        format!("{} ms", elapsed.as_millis())
    } else {
        format!("{} s", elapsed.as_secs_f64())
    };

    info!("Shell initialization took: {}", elapsed_string);

    enter_repl()?;

    let history_file = init::get_user_cache_dir()?.join(".history");
    input::save_history(&history_file)?;

    eprintln!("quit");

    Ok(())
}

fn enter_repl() -> anyhow::Result<()> {
    let history_file = init::get_user_cache_dir()?.join(".history");
    let _ = File::create_new(&history_file);

    input::load_history(&history_file)?;

    // Enter main loop
    loop {
        let prompt = executor::execute_command("rush-prompt", RVec::new()).message;

        match input::readline(&prompt) {
            Ok(line) => {
                input::add_history(&line)?;
                executor::execute_user_input(&line);
            }
            Err(ReadlineError::Interrupted) => {
                eprintln!("^C");
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(e) => {
                error!("{}", e);
                break;
            }
        }
    }

    Ok(()) // It's not ok but fine, we gonna handle later
}
