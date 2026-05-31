use std::{fs, fs::File, path::PathBuf};

use env_logger::{Builder, Env};
use log::{debug, error};
use rustyline::error::ReadlineError;

use crate::{executor::Executor, input::InputHandler};

pub use rush_interface::ExecResult;

pub mod env;
pub mod executor;
mod input;
pub mod lexer;
pub mod parser;
pub mod plugin;
pub mod shell_builtins;
pub mod types;
pub mod user;

/// Parse and execute a string of shell source (used by REPL and scripts).
pub fn execute_string(executor: &Executor, input: &str) -> anyhow::Result<ExecResult> {
    let lexer = lexer::Lexer::new(input);
    let tokens = lexer.tokenize();
    let program = parser::parse(&tokens)?;
    Ok(executor.execute_program(&program))
}

/// Initialise the shell runtime (logger, env, plugins, builtins).
pub fn init_runtime() -> anyhow::Result<(user::UserDirectoryRegistry, Executor)> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let user_dirs = user::init_module()?;
    let env = env::init_module(&user_dirs)?;
    let executor =
        executor::init_module(shell_builtins::init_module()?, plugin::init_module(&env)?)?;

    Ok((user_dirs, executor))
}

/// Run a shell script from a file.
pub fn run_script(path: &str) -> anyhow::Result<()> {
    let (_user_dirs, executor) = init_runtime()?;
    let source = fs::read_to_string(path)?;
    execute_string(&executor, &source)?;
    Ok(())
}

/// Start the interactive REPL.
pub fn start_shell() -> anyhow::Result<()> {
    let (user_dirs, executor) = init_runtime()?;

    let mut input_handler = InputHandler::new()?;
    input_handler.set_commands(executor.command_names());

    let history_file = PathBuf::from(user_dirs.get_cache_dir()).join(".history");
    if let Err(e) = File::create_new(&history_file) {
        debug!(
            "Failed to create history file: {}. Error: {e}",
            history_file.display()
        )
    }

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

    loop {
        let prompt = executor
            .execute_command(types::Command::new("rush-prompt"))
            .message;

        let result = input_handler.readline(&prompt);

        match result {
            Ok(line) => {
                input_handler.add_history(&line)?;

                match execute_string(executor, &line) {
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
