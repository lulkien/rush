/// Integration test for pipe output correctness.
/// Exercises the executor directly, bypassing the REPL.
use rush::executor::Executor;
use rush::types::{Command, CommandPipe, CommandPipeList};

fn setup() -> Executor {
    let user_dirs = rush::user::init_module().unwrap();
    let env = rush::env::init_module(&user_dirs).unwrap();
    let builtins = rush::shell_builtins::init_module().unwrap();
    let plugins = rush::plugin::init_module(&env).unwrap();
    Executor::new(builtins, plugins)
}

/// Run a pipe list and capture what gets written to stdout.
fn capture_stdout(executor: &Executor, pipe_list: CommandPipeList) -> String {
    use std::io::Read;
    use std::os::fd::FromRawFd;

    let mut fds = [0i32; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let (r, w): (i32, i32) = (fds[0], fds[1]);

    let saved_stdout = unsafe { libc::dup(libc::STDOUT_FILENO) };
    assert_ne!(saved_stdout, -1, "dup failed");
    assert_ne!(unsafe { libc::dup2(w, libc::STDOUT_FILENO) }, -1, "dup2 failed");
    unsafe { libc::close(w); }

    // Build a Program from the flat pipe list and execute it.
    let program = rush::types::Program {
        items: pipe_list
            .into_iter()
            .map(|pipe| {
                let commands: Vec<Command> = pipe.into_iter().collect();
                rush::types::CompleteCommand {
                    list: rush::types::AndOrList {
                        first: rush::types::Pipeline {
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

    // Restore stdout before reading
    assert_ne!(
        unsafe { libc::dup2(saved_stdout, libc::STDOUT_FILENO) },
        -1,
        "restore dup2 failed"
    );
    unsafe { libc::close(saved_stdout); }

    let mut output = String::new();
    let reader = unsafe { std::fs::File::from_raw_fd(r) };
    let mut bufreader = std::io::BufReader::new(reader);
    bufreader.read_to_string(&mut output).unwrap();
    output
}

fn make_cmd(name: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(name);
    for a in args {
        cmd.args.push((*a).into());
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
    // echo -n produces "hello" (no \n). print_result adds one.
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
    // echo → cat → cat: should still be exactly "hello\n"
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
