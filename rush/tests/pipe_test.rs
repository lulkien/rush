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
/// We redirect stdout (fd 1) to a pipe and read it back.
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

    executor.execute_command_pipe_list(pipe_list);

    // Restore stdout before reading
    assert_ne!(unsafe { libc::dup2(saved_stdout, libc::STDOUT_FILENO) }, -1, "restore dup2 failed");
    unsafe { libc::close(saved_stdout); }

    let mut output = String::new();
    let reader = unsafe { std::fs::File::from_raw_fd(r) };
    let mut bufreader = std::io::BufReader::new(reader);
    bufreader.read_to_string(&mut output).unwrap();
    output
}

#[test]
fn test_echo_hello() {
    let executor = setup();
    let mut pipe = CommandPipe::new();
    pipe.append_command(Command::new_with_args(
        "echo".into(),
        vec!["hello".into()].into(),
    ));

    let mut list = CommandPipeList::new();
    list.append_pipe(pipe);

    let output = capture_stdout(&executor, list);
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_echo_hello_pipe_cat() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(Command::new_with_args(
        "echo".into(),
        vec!["hello".into()].into(),
    ));
    pipe.append_command(Command::new("cat".into()));

    let mut list = CommandPipeList::new();
    list.append_pipe(pipe);

    let output = capture_stdout(&executor, list);
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_echo_no_trailing_newline() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(Command::new_with_args(
        "echo".into(),
        vec!["-n".into(), "hello".into()].into(),
    ));

    let mut list = CommandPipeList::new();
    list.append_pipe(pipe);

    let output = capture_stdout(&executor, list);
    // echo -n produces "hello" (no \n). print_result adds one.
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_pipe_three_commands() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(Command::new_with_args(
        "echo".into(),
        vec!["hello".into()].into(),
    ));
    pipe.append_command(Command::new("cat".into()));
    pipe.append_command(Command::new("cat".into()));

    let mut list = CommandPipeList::new();
    list.append_pipe(pipe);

    let output = capture_stdout(&executor, list);
    // echo → cat → cat: should still be exactly "hello\n"
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_pipe_with_echo_n() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(Command::new_with_args(
        "echo".into(),
        vec!["-n".into(), "hello".into()].into(),
    ));
    pipe.append_command(Command::new("cat".into()));

    let mut list = CommandPipeList::new();
    list.append_pipe(pipe);

    let output = capture_stdout(&executor, list);
    // echo -n produces "hello" (no \n), print_result adds one → pipe gets "hello\n"
    // cat reads "hello\n", returns it, print_result adds nothing (already ends with \n)
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_echo_empty() {
    let executor = setup();

    let mut pipe = CommandPipe::new();
    pipe.append_command(Command::new_with_args(
        "echo".into(),
        vec!["-n".into()].into(),
    ));

    let mut list = CommandPipeList::new();
    list.append_pipe(pipe);

    let output = capture_stdout(&executor, list);
    // echo -n with no args produces empty message. print_result writes nothing.
    assert_eq!(output, "", "got: {output:?}");
}
