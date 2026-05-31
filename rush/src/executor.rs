use std::os::fd::{BorrowedFd, OwnedFd, RawFd};

use dashmap::DashMap;
use nix::{
    fcntl::{FcntlArg, FdFlag, fcntl},
    sys::wait::{waitpid, WaitStatus},
    unistd::{ForkResult, Pid, fork, pipe, write},
};
use rush_interface::ExecResult;

use crate::{
    plugin::PluginRegistry,
    shell_builtins::BuiltinsRegistry,
    types::{Command, CommandKind, DashRegistry, Program},
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

    pub fn execute_program(&self, program: &Program) -> ExecResult {
        let mut last_result = ExecResult::default();

        for item in &program.items {
            if item.background {
                eprintln!("rush: background execution not yet implemented");
            }

            let list = &item.list;
            last_result = self.execute_pipeline(&list.first);

            for (op, pipeline) in &list.rest {
                let success = last_result.code == 0;
                let should_run = match op {
                    crate::types::AndOr::And => success,
                    crate::types::AndOr::Or => !success,
                };
                if should_run {
                    last_result = self.execute_pipeline(pipeline);
                }
            }
        }

        last_result
    }

    fn execute_pipeline(&self, pipeline: &crate::types::Pipeline) -> ExecResult {
        let commands: Vec<&Command> = pipeline.commands.iter().collect();

        if commands.is_empty() {
            return ExecResult::default();
        }

        match commands.len() {
            1 => {
                let cmd = commands[0];
                match &cmd.kind {
                    CommandKind::Simple => {
                        let result = self.execute_single(cmd.clone());
                        if cmd.redirects.is_empty() {
                            print_result(&result);
                        }
                        result
                    }
                    _ => {
                        eprintln!(
                            "rush: compound commands not yet implemented in executor"
                        );
                        ExecResult::new(1, "")
                    }
                }
            }
            _ => {
                let owned: Vec<Command> = commands.into_iter().cloned().collect();
                self.execute_pipe_forked(owned)
            }
        }
    }

    pub fn execute_command(&self, command: Command) -> ExecResult {
        self.execute_single(command)
    }

    /// Collect all registered command names (builtins + plugins).
    pub fn command_names(&self) -> Vec<String> {
        let mut names = self.builtin_reg.names();
        names.extend(self.plugin_reg.names());
        names.sort();
        names.dedup();
        names
    }

    fn execute_single(&self, command: Command) -> ExecResult {
        if command.name.is_empty() {
            return ExecResult::default();
        }

        if let Some(cache_entry) = self.entry_point_cache.get(command.name.as_str()) {
            return match cache_entry.value() {
                ExecutionFrom::Builtin => self.builtin_reg.execute(command),
                ExecutionFrom::Plugin => self.plugin_reg.execute(command),
                ExecutionFrom::NotFound => ExecResult::new(
                    127,
                    format!("{}: command not found", command.name.as_str()).as_str(),
                ),
            };
        }

        self.lookup_and_execute(command)
    }

    fn lookup_and_execute(&self, command: Command) -> ExecResult {
        if self.plugin_reg.contains(&command.name) {
            self.entry_point_cache
                .insert(command.name.to_string(), ExecutionFrom::Plugin);
            self.plugin_reg.execute(command)
        } else if self.builtin_reg.contains(&command.name) {
            self.entry_point_cache
                .insert(command.name.to_string(), ExecutionFrom::Builtin);
            self.builtin_reg.execute(command)
        } else {
            self.entry_point_cache
                .insert(command.name.to_string(), ExecutionFrom::NotFound);
            ExecResult::new(
                127,
                format!("{}: command not found", command.name.as_str()).as_str(),
            )
        }
    }

    fn execute_pipe_forked(&self, commands: Vec<Command>) -> ExecResult {
        let n = commands.len();

        let mut pipes: Vec<(OwnedFd, OwnedFd)> = Vec::with_capacity(n.saturating_sub(1));
        for _ in 0..n.saturating_sub(1) {
            match pipe() {
                Ok((r, w)) => {
                    let _ = fcntl(&r, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC));
                    let _ = fcntl(&w, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC));
                    pipes.push((r, w));
                }
                Err(e) => {
                    return ExecResult::new(1, &format!("pipe() failed: {e}"));
                }
            }
        }

        let mut pids: Vec<Pid> = Vec::with_capacity(n);
        // Borrow raw stdin/stdout once for reuse across loop iterations.
        let stdin_fd = raw_fd(nix::libc::STDIN_FILENO);
        let stdout_fd = raw_fd(nix::libc::STDOUT_FILENO);

        for i in 0..n {
            // SAFETY: Rush is single-threaded; each child exits immediately.
            match unsafe { fork() } {
                Ok(ForkResult::Child) => {
                    if i > 0 {
                        let mut target = nix::unistd::dup(stdin_fd).expect("dup stdin");
                        nix::unistd::dup2(&pipes[i - 1].0, &mut target).expect("dup2 stdin");
                    }
                    if i < n - 1 {
                        let mut target = nix::unistd::dup(stdout_fd).expect("dup stdout");
                        nix::unistd::dup2(&pipes[i].1, &mut target).expect("dup2 stdout");
                    }
                    drop(pipes);

                    let result = self.lookup_and_execute(commands[i].clone());
                    print_result(&result);
                    std::process::exit(result.code as i32);
                }
                Ok(ForkResult::Parent { child }) => pids.push(child),
                Err(e) => {
                    return ExecResult::new(1, &format!("fork() failed: {e}"));
                }
            }
        }

        drop(pipes);

        let mut last_code = 0i32;
        for pid in &pids {
            if let Ok(WaitStatus::Exited(_, code)) = waitpid(*pid, None) {
                last_code = code;
            }
        }

        ExecResult::new(last_code as u8, "")
    }
}

/// Write a byte slice to a raw fd, handling partial writes and EINTR.
fn write_all(fd: RawFd, mut data: &[u8]) {
    let fd_borrowed = raw_fd(fd);
    while !data.is_empty() {
        match write(fd_borrowed, data) {
            Ok(n) if n > 0 => data = &data[n..],
            _ => break,
        }
    }
}

fn print_result(result: &ExecResult) {
    let fd: RawFd = if result.code == 0 { 1 } else { 2 };
    if !result.message.is_empty() {
        let msg = result.message.as_bytes();
        write_all(fd, msg);
        if !msg.ends_with(b"\n") {
            write_all(fd, b"\n");
        }
    }
}

/// Create a `BorrowedFd` from a raw fd number.
fn raw_fd(fd: RawFd) -> BorrowedFd<'static> {
    // SAFETY: fd is a valid open file descriptor (standard fds or our own).
    unsafe { BorrowedFd::borrow_raw(fd) }
}

pub fn init_module(
    builtin_reg: BuiltinsRegistry,
    plugin_reg: PluginRegistry,
) -> anyhow::Result<Executor> {
    let executor = Executor::new(builtin_reg, plugin_reg);
    Ok(executor)
}
