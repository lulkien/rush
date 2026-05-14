use dashmap::DashMap;
use rush_interface::ExecResult;

use crate::{
    plugin::PluginRegistry,
    shell_builtins::BuiltinsRegistry,
    types::{Command, CommandPipe, CommandPipeList, DashRegistry},
};

enum ExecutionFrom {
    Builtin,
    Plugin,
    NotFound,
}

pub struct Executor {
    builtin_reg: BuiltinsRegistry,
    plugin_reg: PluginRegistry,
    entry_point_cache: DashMap<String, ExecutionFrom>,
}

impl Executor {
    pub fn new(builtin_reg: BuiltinsRegistry, plugin_reg: PluginRegistry) -> Self {
        Self {
            builtin_reg,
            plugin_reg,
            entry_point_cache: DashMap::default(),
        }
    }

    pub fn execute_command_pipe_list(&self, pipes: CommandPipeList) -> ExecResult {
        let mut last_result = ExecResult::default();

        pipes.into_iter().for_each(|pipe| {
            last_result = self.execute_pipe(pipe);
        });

        last_result
    }

    fn execute_pipe(&self, pipe: CommandPipe) -> ExecResult {
        let mut last_result = ExecResult::default();

        pipe.into_iter().for_each(|command| {
            last_result = self.execute_command_with_result(command, last_result.clone());
        });

        if last_result.code == 0 {
            println!("{}", last_result.message);
        } else {
            eprintln!("{}", last_result.message);
        }

        last_result
    }

    pub fn execute_command(&self, command: Command) -> ExecResult {
        self.execute_command_with_result(command, ExecResult::default())
    }

    fn execute_command_with_result(&self, command: Command, last_result: ExecResult) -> ExecResult {
        if let Some(cache_entry) = self.entry_point_cache.get(command.name.as_str()) {
            match cache_entry.value() {
                ExecutionFrom::Builtin => {
                    return self.builtin_reg.execute(command, last_result);
                }
                ExecutionFrom::Plugin => {
                    return self.plugin_reg.execute(command, last_result);
                }
                ExecutionFrom::NotFound => {
                    return ExecResult::new(
                        1,
                        format!("{}: Command not found", command.name.as_str()).as_str(),
                    );
                }
            }
        }

        self.lookup_and_execute(command, last_result)
    }

    fn lookup_and_execute(&self, command: Command, last_result: ExecResult) -> ExecResult {
        if self.plugin_reg.contains(&command.name) {
            self.entry_point_cache
                .insert(command.name.to_string(), ExecutionFrom::Plugin);
            self.plugin_reg.execute(command, last_result)
        } else if self.builtin_reg.contains(&command.name) {
            self.entry_point_cache
                .insert(command.name.to_string(), ExecutionFrom::Builtin);
            self.builtin_reg.execute(command, last_result)
        } else {
            self.entry_point_cache
                .insert(command.name.to_string(), ExecutionFrom::NotFound);
            ExecResult::new(
                1,
                format!("{}: Command not found", command.name.as_str()).as_str(),
            )
        }
    }
}

pub fn init_module(
    builtin_reg: BuiltinsRegistry,
    plugin_reg: PluginRegistry,
) -> anyhow::Result<Executor> {
    let executor = Executor::new(builtin_reg, plugin_reg);

    Ok(executor)
}
