//! rush-core: the shell engine — lexer, parser, executor, variables, plugins.
//!
//! This crate provides everything needed to parse and execute shell commands.
//! The `rush` binary adds the interactive REPL on top.

use env_logger::{Builder, Env};

use crate::executor::Executor;
use crate::var::VarStore;
pub use rush_interface::CommandResult;

pub mod env;
pub mod executor;
pub mod lexer;
pub mod parser;
pub mod plugin;
pub mod shell_builtins;
pub mod types;
pub mod user;
pub mod var;

use crate::types::{AndOrList, Pipeline};

/// Expand `$VAR` references in a complete command.
fn expand_complete_command(item: &mut types::CompleteCommand, vars: &VarStore) {
    expand_pipeline(&mut item.list.first, vars);
    for (_, pipeline) in &mut item.list.rest {
        expand_pipeline(pipeline, vars);
    }
}

fn expand_pipeline(pipeline: &mut types::Pipeline, vars: &VarStore) {
    for cmd in &mut pipeline.commands {
        expand_command(cmd, vars);
    }
}

fn expand_command(cmd: &mut types::Command, vars: &VarStore) {
    let expanded_name = vars.expand_string(&cmd.name);
    if expanded_name != *cmd.name {
        cmd.name = expanded_name.to_string();
    }
    for arg in cmd.args.iter_mut() {
        if arg.starts_with('\x01') {
            *arg = arg[1..].to_string();
            continue;
        }
        let expanded = vars.expand_string(arg);
        if expanded != **arg {
            *arg = expanded.to_string();
        }
    }
}

/// Parse and execute a string of shell source.
pub fn execute_string(
    executor: &Executor,
    vars: &var::VarStore,
    input: &str,
) -> anyhow::Result<CommandResult> {
    let lexer = lexer::Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut program = parser::parse(&tokens)?;

    preprocess(&mut program, vars);

    let mut last_result = CommandResult::default();
    for item in &mut program.items {
        expand_complete_command(item, vars);

        let list = std::mem::replace(&mut item.list, AndOrList {
            first: Pipeline { negation: false, commands: vec![] },
            rest: vec![],
        });

        last_result = executor.execute_pipeline(&list.first);
        vars.set_exit_code(last_result.code);

        for (op, pipeline) in &list.rest {
            let success = last_result.code == 0;
            let should_run = match op {
                crate::types::AndOr::And => success,
                crate::types::AndOr::Or => !success,
            };
            if should_run {
                last_result = executor.execute_pipeline(pipeline);
                vars.set_exit_code(last_result.code);
            }
        }
    }

    Ok(last_result)
}

/// Extract assignments from the AST.
fn preprocess(program: &mut types::Program, vars: &var::VarStore) {
    for item in &mut program.items {
        let list = &mut item.list;
        preprocess_pipeline(&mut list.first, vars);
        for (_, pipeline) in &mut list.rest {
            preprocess_pipeline(pipeline, vars);
        }
    }
}

fn preprocess_pipeline(pipeline: &mut types::Pipeline, vars: &var::VarStore) {
    for cmd in &mut pipeline.commands {
        if let Some((var_name, var_value)) = split_assignment(&cmd.name) {
            vars.set_colon(&var_name, &var_value);
            if cmd.args.is_empty() {
                cmd.name = "true".to_string();
            } else {
                let new_name = cmd.args[0].clone();
                cmd.name = new_name;
                cmd.args.remove(0);
            }
        }
    }
}

fn split_assignment(word: &str) -> Option<(String, String)> {
    if let Some(pos) = word.find('=')
        && pos > 0
    {
        let name = word[..pos].to_string();
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
pub fn init_runtime() -> anyhow::Result<(user::UserDirectoryRegistry, Executor, var::VarStore)> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let user_dirs = user::init_module()?;
    let env = env::init_module(&user_dirs)?;
    let vars = var::VarStore::default();
    let executor =
        executor::init_module(shell_builtins::init_module()?, plugin::init_module(&env)?)?;

    for (key, value) in std::env::vars() {
        vars.set_colon(&key, &value);
    }

    Ok((user_dirs, executor, vars))
}
