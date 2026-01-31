use rush::start_shell;

fn main() {
    if let Err(e) = start_shell() {
        eprintln!("{e}");
    }
}
