use std::{fs::File, path::PathBuf};

use env_logger::{Builder, Env};
use log::{debug, error};
use rustyline::error::ReadlineError;

use crate::{executor::Executor, input::InputHandler, lexer::Lexer};
pub use rush_interface::ExecResult;

pub mod env;
pub mod executor;
mod input;
mod lexer;
pub mod plugin;
pub mod shell_builtins;
pub mod types;
pub mod user;

pub fn start_shell() -> anyhow::Result<()> {
    // Init logger
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let user_dirs = user::init_module()?;
    let env = env::init_module(&user_dirs)?;

    let executor =
        executor::init_module(shell_builtins::init_module()?, plugin::init_module(&env)?)?;

    let mut input_handler = InputHandler::new()?;

    let history_file = PathBuf::from(user_dirs.get_cache_dir()).join(".history");
    if let Err(e) = File::create_new(&history_file) {
        debug!(
            "Failed to create history file: {}. Error: {e}",
            history_file.display()
        )
    }

    // Enter main loop
    enter_repl(&mut input_handler, &history_file, &executor)?;

    eprintln!("Bye bye");
    Ok(())
}

fn enter_repl(
    input_handler: &mut InputHandler,
    history_file: &PathBuf,
    executor: &Executor,
) -> anyhow::Result<()> {
    input_handler.load_history(history_file)?;

    // Enter main loop
    loop {
        let prompt = executor
            .execute_command(types::Command::new("rush-prompt"))
            .message;

        let result = input_handler.readline(&prompt);

        match result {
            Ok(line) => {
                input_handler.add_history(&line)?;

                let pipe_list = match Lexer::new(&line).parse_line() {
                    Ok(list) => list,
                    Err(e) => {
                        eprintln!("{e}");
                        continue;
                    }
                };

                let _result = executor.execute_command_pipe_list(pipe_list);
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
