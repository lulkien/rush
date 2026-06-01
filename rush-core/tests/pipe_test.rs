/// Integration test for pipe output correctness.
use rush_core::executor::Executor;
use rush_core::types::{Command, Pipeline};

fn setup() -> Executor {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let data_dir = format!("{home}/.local/share/rush");
    let vars = std::rc::Rc::new(rush_core::var::VarStore::default());
    vars.set("RUSH_DATA_PATH", vec![data_dir]);
    vars.set("RUSH_PLUGIN_PATH", Vec::new());
    let builtins = rush_core::shell_builtins::init_module().unwrap();
    let plugins = rush_core::plugin::init_module(&vars).unwrap();
    Executor::new(builtins, plugins, vars)
}

/// Run a pipeline and capture stdout.
fn capture_stdout(executor: &Executor, pipeline: &Pipeline) -> String {
    use nix::unistd;
    use std::io::Read;
    use std::os::fd::{BorrowedFd, FromRawFd, OwnedFd};

    fn raw_fd(fd: std::os::fd::RawFd) -> BorrowedFd<'static> {
        unsafe { BorrowedFd::borrow_raw(fd) }
    }

    let stdout = raw_fd(nix::libc::STDOUT_FILENO);

    let (r, w) = unistd::pipe().expect("pipe failed");
    let saved = unistd::dup(stdout).expect("dup stdout");
    let mut stdout_fd = unsafe { OwnedFd::from_raw_fd(nix::libc::STDOUT_FILENO) };
    unistd::dup2(&w, &mut stdout_fd).expect("dup2 stdout → pipe");
    std::mem::forget(stdout_fd);
    drop(w);

    executor.execute_pipeline(pipeline);

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

#[test]
fn test_echo_hello() {
    let executor = setup();
    let pipeline = Pipeline {
        negation: false,
        commands: vec![make_cmd("echo", &["hello"])],
    };
    let output = capture_stdout(&executor, &pipeline);
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_echo_hello_pipe_cat() {
    let executor = setup();
    let pipeline = Pipeline {
        negation: false,
        commands: vec![
            make_cmd("echo", &["hello"]),
            make_cmd("cat", &[]),
        ],
    };
    let output = capture_stdout(&executor, &pipeline);
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_echo_no_trailing_newline() {
    let executor = setup();
    let pipeline = Pipeline {
        negation: false,
        commands: vec![make_cmd("echo", &["-n", "hello"])],
    };
    let output = capture_stdout(&executor, &pipeline);
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_pipe_three_commands() {
    let executor = setup();
    let pipeline = Pipeline {
        negation: false,
        commands: vec![
            make_cmd("echo", &["hello"]),
            make_cmd("cat", &[]),
            make_cmd("cat", &[]),
        ],
    };
    let output = capture_stdout(&executor, &pipeline);
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_pipe_with_echo_n() {
    let executor = setup();
    let pipeline = Pipeline {
        negation: false,
        commands: vec![
            make_cmd("echo", &["-n", "hello"]),
            make_cmd("cat", &[]),
        ],
    };
    let output = capture_stdout(&executor, &pipeline);
    assert_eq!(output, "hello\n", "got: {output:?}");
}

#[test]
fn test_echo_empty() {
    let executor = setup();
    let pipeline = Pipeline {
        negation: false,
        commands: vec![make_cmd("echo", &["-n"])],
    };
    let output = capture_stdout(&executor, &pipeline);
    assert_eq!(output, "", "got: {output:?}");
}
