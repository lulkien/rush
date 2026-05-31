use std::os::fd::{AsRawFd, OwnedFd, RawFd};

use dashmap::DashMap;
use nix::{
    sys::wait::{waitpid, WaitStatus},
    unistd::{ForkResult, Pid, fork, pipe},
};
use rush_interface::ExecResult;

use crate::{
    plugin::PluginRegistry,
    shell_builtins::BuiltinsRegistry,
    types::{Command, CommandPipeList, DashRegistry},
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

    /// Execute a list of semicolon-separated pipe groups.
    /// Single-command groups run in-process.
    /// Multi-command groups fork N children connected by N-1 Unix pipes.
    pub fn execute_command_pipe_list(&self, pipes: CommandPipeList) -> ExecResult {
        let mut last_result = ExecResult::default();

        for pipe in pipes {
            let commands: Vec<Command> = pipe.into_iter().collect();
            last_result = match commands.len() {
                0 => ExecResult::default(),
                1 => {
                    let result = self.execute_single(commands.into_iter().next().unwrap());
                    print_result(&result);
                    result
                }
                _ => self.execute_pipe_forked(commands),
            };
        }

        last_result
    }

    /// Run a single command in-process (no fork).
    /// Used by the REPL for the prompt plugin.
    pub fn execute_command(&self, command: Command) -> ExecResult {
        self.execute_single(command)
    }

    /// Look up and execute a single command via the registry.
    fn execute_single(&self, command: Command) -> ExecResult {
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

    /// Fork N child processes, connect them with N-1 Unix pipes,
    /// wait for all children, return the last command's exit code.
    ///
    /// Like fish's execution engine, pipe fds are marked FD_CLOEXEC
    /// to prevent leaks to grandchild processes.
    ///
    /// # Safety
    ///
    /// `fork()` is unsafe in multi-threaded programs. Rush is
    /// single-threaded; each child runs one command then exits.
    fn execute_pipe_forked(&self, commands: Vec<Command>) -> ExecResult {
        let n = commands.len();

        // Create N-1 pipes, mark each fd CLOEXEC (fish pattern).
        let mut pipes: Vec<(OwnedFd, OwnedFd)> = Vec::with_capacity(n.saturating_sub(1));
        for _ in 0..n.saturating_sub(1) {
            match pipe() {
                Ok((r, w)) => {
                    set_cloexec(r.as_raw_fd());
                    set_cloexec(w.as_raw_fd());
                    pipes.push((r, w));
                }
                Err(e) => {
                    return ExecResult::new(1, &format!("pipe() failed: {e}"));
                }
            }
        }

        let mut pids: Vec<Pid> = Vec::with_capacity(n);

        for i in 0..n {
            match unsafe { fork() } {
                Ok(ForkResult::Child) => {
                    // ── child ──────────────────────────────────
                    // Wire stdin from previous pipe, stdout to next pipe.
                    if i > 0 {
                        unsafe { libc::dup2(pipes[i - 1].0.as_raw_fd(), libc::STDIN_FILENO); }
                    }
                    if i < n - 1 {
                        unsafe { libc::dup2(pipes[i].1.as_raw_fd(), libc::STDOUT_FILENO); }
                    }
                    // Drop OwnedFds — closes every pipe fd the child
                    // replaced with dup2 or doesn't need.
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

        // ── parent ──────────────────────────────────────────
        // Close all pipe fds BEFORE waiting. If the parent still
        // holds the write end, the reading child (e.g. cat) never
        // sees EOF and blocks forever in read_to_string.
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

/// Mark an fd close-on-exec so it doesn't leak through future exec calls.
fn set_cloexec(fd: RawFd) {
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFD);
        if flags != -1 {
            libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
        }
    }
}

/// Write a byte slice to a raw fd, handling partial writes and EINTR.
/// Like fish's `write_loop` — no Rust stdio buffering, fork-safe.
fn write_all(fd: RawFd, mut data: &[u8]) {
    while !data.is_empty() {
        let n = unsafe { libc::write(fd, data.as_ptr() as *const _, data.len()) };
        if n > 0 {
            data = &data[n as usize..];
        } else if n == 0 || std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) {
            break;
        }
    }
}

/// Print result directly to fd 1 (stdout) or fd 2 (stderr).
/// Uses raw `write` syscalls — no `println!`/`eprintln!` buffering.
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

pub fn init_module(
    builtin_reg: BuiltinsRegistry,
    plugin_reg: PluginRegistry,
) -> anyhow::Result<Executor> {
    let executor = Executor::new(builtin_reg, plugin_reg);
    Ok(executor)
}
