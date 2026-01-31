use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use abi_stable::std_types::{RString, RVec};
use rush_interface::ExecResult;

use crate::{builtins, executor::ExecutorWrapper};

pub struct ShellCommands {
    prompt: ExecutorWrapper,
    exit: ExecutorWrapper,
}

static SHELL_COMMANDS: OnceLock<RwLock<ShellCommands>> = OnceLock::new();

impl Default for ShellCommands {
    fn default() -> Self {
        Self {
            prompt: ExecutorWrapper::new(builtins::prompt),
            exit: ExecutorWrapper::new(builtins::exit),
        }
    }
}

impl ShellCommands {
    pub fn set_prompt(&mut self, prompt: ExecutorWrapper) {
        self.prompt = prompt;
    }

    pub fn execute_command(&self, cmd: &str, args: RVec<RString>) -> ExecResult {
        match cmd {
            "rush_prompt" => self.prompt.exec(args),
            "exit" => self.exit.exec(args),
            _ => ExecResult::new(127, &format!("{}: command not found", cmd)),
        }
    }
}

fn get_shell_commands() -> &'static RwLock<ShellCommands> {
    SHELL_COMMANDS.get_or_init(|| RwLock::new(ShellCommands::default()))
}

pub fn read_shell_commands() -> anyhow::Result<RwLockReadGuard<'static, ShellCommands>> {
    get_shell_commands()
        .read()
        .map_err(|_| anyhow::anyhow!("Shell commands read lock poisoned"))
}

pub fn write_shell_commands() -> anyhow::Result<RwLockWriteGuard<'static, ShellCommands>> {
    get_shell_commands()
        .write()
        .map_err(|_| anyhow::anyhow!("Shell commands write lock poisoned"))
}
