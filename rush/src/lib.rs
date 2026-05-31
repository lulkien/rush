use std::{fs, fs::File, path::PathBuf};

use env_logger::{Builder, Env};
use log::{debug, error};
use rustyline::error::ReadlineError;

use crate::{executor::Executor, input::InputHandler, var::VarStore};

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
pub mod var;

/// Expand `$VAR` references in command words and handle assignments.
fn expand_and_assign(program: &mut types::Program, vars: &VarStore, last_exit: u8) {
    // Track the last exit code for $? expansion.
    // We can't easily propagate this in a pre-pass since it depends on
    // execution order, so we use the passed-in value for the single-command
    // case (REPL) and default to 0 for scripts.
    let _ = last_exit;

    for item in &mut program.items {
        let list = &mut item.list;
        // Expand in first pipeline
        expand_pipeline(&mut list.first, vars);
        // Expand in rest
        for (_, pipeline) in &mut list.rest {
            expand_pipeline(pipeline, vars);
        }
    }
}

fn expand_pipeline(pipeline: &mut types::Pipeline, vars: &VarStore) {
    for cmd in &mut pipeline.commands {
        expand_command(cmd, vars);
    }
}

fn expand_command(cmd: &mut types::Command, vars: &VarStore) {
    // Expand $VAR in the command name
    let expanded_name = vars.expand_string(&cmd.name, 0);
    if expanded_name != *cmd.name {
        cmd.name = expanded_name.into();
    }
    // Expand $VAR in args
    for arg in cmd.args.iter_mut() {
        let expanded = vars.expand_string(arg, 0);
        if expanded != **arg {
            *arg = expanded.into();
        }
    }
}

/// Parse and execute a string of shell source (used by REPL and scripts).
pub fn execute_string(
    executor: &Executor,
    vars: &VarStore,
    input: &str,
) -> anyhow::Result<ExecResult> {
    let lexer = lexer::Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut program = parser::parse(&tokens)?;

    // Pre-pass: handle assignments and expand variables.
    preprocess(&mut program, vars);

    // Expand $VAR references across the program.
    expand_and_assign(&mut program, vars, 0);

    Ok(executor.execute_program(&program))
}

/// Pre-process: extract assignments from the AST.
/// `VAR=val` → store in VarStore, clear the command.
/// `VAR=val cmd args` → store, transform command to `cmd args`.
fn preprocess(program: &mut types::Program, vars: &VarStore) {
    for item in &mut program.items {
        let list = &mut item.list;
        preprocess_pipeline(&mut list.first, vars);
        for (_, pipeline) in &mut list.rest {
            preprocess_pipeline(pipeline, vars);
        }
    }
}

fn preprocess_pipeline(pipeline: &mut types::Pipeline, vars: &VarStore) {
    // Only preprocess single-command pipelines for assignment detection.
    // For pipelines like `VAR=val cmd1 | cmd2`, VAR is set for the whole
    // pipeline in POSIX, but we handle the simple case for now.

    for cmd in &mut pipeline.commands {
        if let Some((var_name, var_value)) = split_assignment(&cmd.name) {
            // Store the variable
            vars.set_colon(&var_name, &var_value);

            if cmd.args.is_empty() {
                // Standalone assignment: `VAR=value` → no command to run.
                cmd.name = "true".into();
            } else {
                // Assignment with command: `VAR=value cmd args...`
                // Shift args: first arg becomes the new command name.
                let new_name = cmd.args[0].clone();
                cmd.name = new_name;
                cmd.args.remove(0);
            }
        }
    }
}

/// Check if a word is an assignment (`NAME=value`).
/// Returns `Some((name, value))` if it is.
fn split_assignment(word: &str) -> Option<(String, String)> {
    // Must contain '=' but not start with it.
    if let Some(pos) = word.find('=')
        && pos > 0
    {
        let name = word[..pos].to_string();
        // Validate: NAME must be alphanumeric + underscore, starting with alpha or underscore.
        if name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            let value = word[pos + 1..].to_string();
            return Some((name, value));
        }
    }
    None
}

/// Initialise the shell runtime (logger, env, plugins, builtins, vars).
pub fn init_runtime() -> anyhow::Result<(user::UserDirectoryRegistry, Executor, VarStore)> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let user_dirs = user::init_module()?;
    let env = env::init_module(&user_dirs)?;
    let executor =
        executor::init_module(shell_builtins::init_module()?, plugin::init_module(&env)?)?;
    let vars = VarStore::default();

    // Seed shell variables from the process environment.
    for (key, value) in std::env::vars() {
        vars.set_colon(&key, &value);
    }

    Ok((user_dirs, executor, vars))
}

/// Run a shell script from a file.
pub fn run_script(path: &str) -> anyhow::Result<()> {
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
    executor: &Executor,
    vars: &VarStore,
) -> anyhow::Result<()> {
    input_handler.load_history(history_file)?;

    let mut _last_exit: u8 = 0;

    loop {
        let prompt = executor
            .execute_command(types::Command::new("rush-prompt"))
            .message;

        let result = input_handler.readline(&prompt);

        match result {
            Ok(line) => {
                let trimmed = line.trim();

                if trimmed == "history-search" || trimmed == "hf" {
                    // Launch the interactive history TUI.
                    if let Some(selected) = input_handler.history_search() {
                        println!("{selected}");
                        input_handler.add_history(&selected)?;
                        match execute_string(executor, vars, &selected) {
                            Ok(result) => _last_exit = result.code,
                            Err(e) => eprintln!("rush: {e}"),
                        }
                    }
                    continue;
                }

                input_handler.add_history(&line)?;

                match execute_string(executor, vars, &line) {
                    Ok(result) => _last_exit = result.code,
                    Err(e) => eprintln!("rush: {e}"),
                }
            }
            Err(ReadlineError::Interrupted) => {
                eprintln!("^C");
                _last_exit = 130;
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
