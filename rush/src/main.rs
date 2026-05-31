use clap::Parser;
use rush::run_script;
use rush_core::{execute_string, init_runtime};

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
        if let Err(e) = run_script(&file) {
            eprintln!("rush: {e}");
            std::process::exit(1);
        }
    } else {
        if let Err(e) = rush::start_shell() {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
