use std::{fs, fs::File, path::PathBuf};

use env_logger::{Builder, Env};
use log::{debug, error};
use rustyline::error::ReadlineError;

use rush_core::{execute_string, init_runtime, types};

mod input;

use crate::input::InputHandler;

/// Run a shell script from a file.
pub fn run_script(path: &str) -> anyhow::Result<()> {
    Builder::from_env(Env::default().default_filter_or("info")).init();
    let (_user_dirs, executor, vars) = init_runtime()?;
    let source = fs::read_to_string(path)?;
    execute_string(&executor, &vars, &source)?;
    Ok(())
}

/// Start the interactive REPL.
pub fn start_shell() -> anyhow::Result<()> {
    let (user_dirs, executor, vars) = init_runtime()?;

    let mut input_handler = InputHandler::new()?;
    input_handler.set_commands(executor.command_names());

    let history_file = PathBuf::from(user_dirs.get_cache_dir()).join(".history");
    if let Err(e) = File::create_new(&history_file) {
        debug!(
            "Failed to create history file: {}. Error: {e}",
            history_file.display()
        )
    }

    enter_repl(&mut input_handler, &history_file, &executor, &vars)?;

    eprintln!("Bye bye");
    Ok(())
}

fn enter_repl(
    input_handler: &mut InputHandler,
    history_file: &PathBuf,
    executor: &rush_core::executor::Executor,
    vars: &rush_core::var::VarStore,
) -> anyhow::Result<()> {
    input_handler.load_history(history_file)?;

    loop {
        let prompt = executor
            .execute_command(types::Command::new("rush-prompt"))
            .message;

        let result = input_handler.readline(&prompt);

        match result {
            Ok(line) => {
                let trimmed = line.trim();

                if trimmed == "history-search" || trimmed == "hf" {
                    if let Some(selected) = input_handler.history_search() {
                        println!("{selected}");
                        input_handler.add_history(&selected)?;
                        match execute_string(executor, vars, &selected) {
                            Ok(_) => {}
                            Err(e) => eprintln!("rush: {e}"),
                        }
                    }
                    continue;
                }

                input_handler.add_history(&line)?;

                match execute_string(executor, vars, &line) {
                    Ok(_) => {}
                    Err(e) => eprintln!("rush: {e}"),
                }
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

    input_handler.save_history(history_file)?;

    Ok(())
}
