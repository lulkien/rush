use std::ffi::CString;
use std::os::fd::{BorrowedFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use dashmap::DashMap;
use nix::{
    fcntl::{FcntlArg, FdFlag, fcntl},
    sys::{
        signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction},
        wait::{WaitStatus, waitpid},
    },
    unistd::{ForkResult, Pid, fork, pipe, write},
};
use rush_interface::CommandResult;

use crate::{
    plugin::PluginRegistry,
    shell_builtins::BuiltinsRegistry,
    types::{Command, CommandKind, DashRegistry, Program},
};

#[derive(Clone)]
enum ExecutionFrom {
    Builtin,
    Plugin,
    External(PathBuf),
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

    pub fn execute_program(&self, program: &Program) -> CommandResult {
        let mut last_result = CommandResult::default();

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

    pub(crate) fn execute_pipeline(&self, pipeline: &crate::types::Pipeline) -> CommandResult {
        let commands: Vec<&Command> = pipeline.commands.iter().collect();

        if commands.is_empty() {
            return CommandResult::default();
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
                        eprintln!("rush: compound commands not yet implemented in executor");
                        CommandResult::new(1, "")
                    }
                }
            }
            _ => {
                let owned: Vec<Command> = commands.into_iter().cloned().collect();
                self.execute_pipe_forked(owned)
            }
        }
    }

    pub fn execute_command(&self, command: Command) -> CommandResult {
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

    fn execute_single(&self, command: Command) -> CommandResult {
        if command.name.is_empty() {
            return CommandResult::default();
        }

        match self.resolve(&command.name) {
            ExecutionFrom::Builtin => self.builtin_reg.execute(command),
            ExecutionFrom::Plugin => self.plugin_reg.execute(command),
            ExecutionFrom::External(_path) => self.execute_external_single(command),
            ExecutionFrom::NotFound => CommandResult::new(
                127,
                format!("{}: command not found", command.name.as_str()).as_str(),
            ),
        }
    }

    /// Resolve a command name and cache the result.
    fn resolve(&self, name: &str) -> ExecutionFrom {
        if let Some(cached) = self.entry_point_cache.get(name) {
            return cached.value().clone();
        }

        let result = if self.plugin_reg.contains(name) {
            ExecutionFrom::Plugin
        } else if self.builtin_reg.contains(name) {
            ExecutionFrom::Builtin
        } else if let Some(path) = find_in_path(name) {
            ExecutionFrom::External(path)
        } else {
            ExecutionFrom::NotFound
        };

        self.entry_point_cache
            .insert(name.to_string(), result.clone());
        result
    }

    /// Fork and exec an external command (single-command, no pipe).
    fn execute_external_single(&self, command: Command) -> CommandResult {
        let (prog, argv) = build_argv(&command);
        let argv_refs: Vec<&std::ffi::CStr> = argv.iter().map(|s| s.as_c_str()).collect();

        // SAFETY: single-threaded; child exits or execs immediately.
        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                // Reset SIGINT to default so Ctrl+C kills this child.
                let sa = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
                let _ = unsafe { sigaction(Signal::SIGINT, &sa) };

                let _ = nix::unistd::execvp(&prog, &argv_refs);
                // execvp only returns on error
                std::process::exit(127);
            }
            Ok(ForkResult::Parent { child }) => match waitpid(child, None) {
                Ok(WaitStatus::Exited(_, code)) => CommandResult::new(code, ""),
                Ok(WaitStatus::Signaled(_, sig, _)) => CommandResult::new(128 + sig as i32, ""),
                _ => CommandResult::new(1, ""),
            },
            Err(e) => CommandResult::new(1, &format!("fork() failed: {e}")),
        }
    }

    /// Execute an external command in the current process (used in pipe children).
    /// Never returns — either execs successfully or exits with 127.
    fn exec_external(&self, command: &Command) -> ! {
        let (prog, argv) = build_argv(command);
        let argv_refs: Vec<&std::ffi::CStr> = argv.iter().map(|s| s.as_c_str()).collect();

        // Reset SIGINT to default.
        let sa = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
        let _ = unsafe { sigaction(Signal::SIGINT, &sa) };

        let _ = nix::unistd::execvp(&prog, &argv_refs);
        // execvp only returns on error
        std::process::exit(127);
    }

    fn execute_pipe_forked(&self, commands: Vec<Command>) -> CommandResult {
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
                    return CommandResult::new(1, &format!("pipe() failed: {e}"));
                }
            }
        }

        let mut pids: Vec<Pid> = Vec::with_capacity(n);

        for i in 0..n {
            // SAFETY: Rush is single-threaded; each child exits immediately.
            match unsafe { fork() } {
                Ok(ForkResult::Child) => {
                    // Reset SIGINT to default so Ctrl+C kills this child.
                    let sa = SigAction::new(SigHandler::SigDfl, SaFlags::empty(), SigSet::empty());
                    let _ = unsafe { sigaction(Signal::SIGINT, &sa) };

                    if i > 0 {
                        // SAFETY: fd 0 is always open; we forget to prevent drop from closing.
                        let mut stdin_fd =
                            unsafe { std::os::fd::OwnedFd::from_raw_fd(nix::libc::STDIN_FILENO) };
                        if nix::unistd::dup2(&pipes[i - 1].0, &mut stdin_fd).is_err() {
                            std::process::exit(1);
                        }
                        std::mem::forget(stdin_fd);
                    }
                    if i < n - 1 {
                        let mut stdout_fd =
                            unsafe { std::os::fd::OwnedFd::from_raw_fd(nix::libc::STDOUT_FILENO) };
                        if nix::unistd::dup2(&pipes[i].1, &mut stdout_fd).is_err() {
                            std::process::exit(1);
                        }
                        std::mem::forget(stdout_fd);
                    }
                    drop(pipes);

                    // External commands: exec directly (no fork inside fork).
                    if matches!(self.resolve(&commands[i].name), ExecutionFrom::External(_)) {
                        self.exec_external(&commands[i]);
                    }

                    let result = self.lookup_and_execute(commands[i].clone());
                    print_result(&result);
                    std::process::exit(result.code);
                }
                Ok(ForkResult::Parent { child }) => pids.push(child),
                Err(e) => {
                    return CommandResult::new(1, &format!("fork() failed: {e}"));
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

        CommandResult::new(last_code, "")
    }

    /// Lookup and execute a builtin or plugin (no PATH search).
    /// Used by pipe children when the command is not external.
    fn lookup_and_execute(&self, command: Command) -> CommandResult {
        match self.resolve(&command.name) {
            ExecutionFrom::Builtin => self.builtin_reg.execute(command),
            ExecutionFrom::Plugin => self.plugin_reg.execute(command),
            _ => CommandResult::new(
                127,
                format!("{}: command not found", command.name.as_str()).as_str(),
            ),
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────

/// Search PATH for an executable named `name`.
fn find_in_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    for dir in path_var.split(':') {
        if dir.is_empty() {
            continue;
        }
        let candidate = PathBuf::from(dir).join(name);
        if let Ok(meta) = std::fs::metadata(&candidate)
            && meta.is_file()
            && meta.permissions().mode() & 0o111 != 0
        {
            return Some(candidate);
        }
    }
    None
}

/// Build argc/argv from a Command for execvp / execv.
/// Returns (program_name, owned_CStrings).
fn build_argv(command: &Command) -> (CString, Vec<CString>) {
    let prog = CString::new(command.name.as_str()).unwrap_or_default();
    let mut argv: Vec<CString> = Vec::with_capacity(1 + command.args.len());
    argv.push(prog.clone());
    for arg in &command.args {
        argv.push(CString::new(arg.as_str()).unwrap_or_default());
    }
    (prog, argv)
}

/// Write a byte slice to a raw fd.
/// Retries on EINTR, returns true if all bytes were written.
fn write_all(fd: RawFd, mut data: &[u8]) -> bool {
    use nix::errno::Errno;
    let fd_borrowed = raw_fd(fd);
    while !data.is_empty() {
        match write(fd_borrowed, data) {
            Ok(0) => return false,
            Ok(n) => data = &data[n..],
            Err(Errno::EINTR) => continue,
            Err(_) => return false,
        }
    }
    true
}

fn print_result(result: &CommandResult) {
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
    // Ignore SIGINT in the shell so Ctrl+C only kills foreground children.
    let sa = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
    unsafe { sigaction(Signal::SIGINT, &sa) }.map_err(|e| anyhow::anyhow!("sigaction: {e}"))?;

    let executor = Executor::new(builtin_reg, plugin_reg);
    Ok(executor)
}
