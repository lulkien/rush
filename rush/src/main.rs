use clap::Parser;
use rush::{execute_string, init_runtime, run_script, start_shell};

/// Rush — a POSIX-compatible shell written in Rust.
#[derive(Parser)]
#[command(name = "rush", version, about)]
struct Cli {
    /// Command string to execute
    #[arg(short = 'c', group = "input")]
    command: Option<String>,

    /// Script file to execute
    #[arg(group = "input")]
    file: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    if let Some(cmd) = cli.command {
        // rush -c "echo hello"
        let (_user_dirs, executor, vars) = match init_runtime() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("rush: {e}");
                std::process::exit(1);
            }
        };
        if let Err(e) = execute_string(&executor, &vars, &cmd) {
            eprintln!("rush: {e}");
            std::process::exit(1);
        }
    } else if let Some(file) = cli.file {
        // rush script.sh
        if let Err(e) = run_script(&file) {
            eprintln!("rush: {e}");
            std::process::exit(1);
        }
    } else {
        // rush (interactive)
        if let Err(e) = start_shell() {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
