/// Integration test for pipe output correctness.
use rush_core::executor::Executor;
use rush_core::types::{Command, CommandPipe, CommandPipeList};

fn setup() -> Executor {
    let user_dirs = rush_core::user::init_module().unwrap();
    let env = rush_core::env::init_module(&user_dirs).unwrap();
    let builtins = rush_core::shell_builtins::init_module().unwrap();
    let plugins = rush_core::plugin::init_module(&env).unwrap();
    let vars = std::rc::Rc::new(rush_core::var::VarStore::default());
    Executor::new(builtins, plugins, vars)
}

/// Run a pipe list and capture what gets written to stdout.
fn capture_stdout(executor: &Executor, pipe_list: CommandPipeList) -> String {
    use nix::unistd;
    use std::io::Read;
    use std::os::fd::{BorrowedFd, FromRawFd, OwnedFd};

    fn raw_fd(fd: std::os::fd::RawFd) -> BorrowedFd<'static> {
        unsafe { BorrowedFd::borrow_raw(fd) }
    }

    let stdout = raw_fd(nix::libc::STDOUT_FILENO);

    let (r, w) = unistd::pipe().expect("pipe failed");
    let saved = unistd::dup(stdout).expect("dup stdout");
    // SAFETY: fd 1 is always open; we mem::forget to prevent drop from closing it.
    let mut stdout_fd = unsafe { OwnedFd::from_raw_fd(nix::libc::STDOUT_FILENO) };
    unistd::dup2(&w, &mut stdout_fd).expect("dup2 stdout → pipe");
    std::mem::forget(stdout_fd);
    drop(w);

    let program = rush_core::types::Program {
        items: pipe_list
            .into_iter()
            .map(|pipe| {
                let commands: Vec<Command> = pipe.into_iter().collect();
                rush_core::types::CompleteCommand {
                    list: rush_core::types::AndOrList {
                        first: rush_core::types::Pipeline {
                            negation: false,
                            commands,
                        },
                        rest: vec![],
                    },
                    background: false,
                }
            })
            .collect(),
    };
    executor.execute_program(&program);

    let mut stdout_fd = unsafe { OwnedFd::from_raw_fd(nix::libc::STDOUT_FILENO) };
    unistd::dup2(&saved, &mut stdout_fd).expect("restore stdout");
    std::mem::forget(stdout_fd);
    drop(saved);

    let mut output = String::new();
    let reader = std::fs::File::from(r);
    let mut bufreader = std::io::BufReader::new(reader);
    bufreader.read_to_string(&mut output).unwrap();
    output
}

fn make_cmd(name: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(name);
    for a in args {
        cmd.args.push((*a).to_string());
    }
    cmd
}

fn single_pipe_list(pipe: CommandPipe) -> CommandPipeList {
    let mut list = CommandPipeList::new();
    list.append_pipe(pipe);
    list
}

#[test]
fn test_echo_hello() {
    let executor = setup();
    let mut pipe = CommandPipe::new();
    pipe.append_command(make_cmd("echo", &["hello"]));

    let output = capture_stdout(&executor, single_pipe_list(pipe));
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_echo_hello_pipe_cat() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(make_cmd("echo", &["hello"]));
    pipe.append_command(make_cmd("cat", &[]));

    let output = capture_stdout(&executor, single_pipe_list(pipe));
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_echo_no_trailing_newline() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(make_cmd("echo", &["-n", "hello"]));

    let output = capture_stdout(&executor, single_pipe_list(pipe));
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_pipe_three_commands() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(make_cmd("echo", &["hello"]));
    pipe.append_command(make_cmd("cat", &[]));
    pipe.append_command(make_cmd("cat", &[]));

    let output = capture_stdout(&executor, single_pipe_list(pipe));
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_pipe_with_echo_n() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(make_cmd("echo", &["-n", "hello"]));
    pipe.append_command(make_cmd("cat", &[]));

    let output = capture_stdout(&executor, single_pipe_list(pipe));
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_echo_empty() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(make_cmd("echo", &["-n"]));

    let output = capture_stdout(&executor, single_pipe_list(pipe));
    assert_eq!(output, "", "got: {output:?}");
}
