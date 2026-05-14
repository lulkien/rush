use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;

#[plugin_name]
pub fn plugin_name() -> RString {
    env!("CARGO_PKG_NAME").into()
}

#[plugin_desc]
pub fn plugin_desc() -> RString {
    env!("CARGO_PKG_DESCRIPTION").into()
}

#[plugin_version]
pub fn plugin_version() -> RString {
    env!("CARGO_PKG_VERSION").into()
}

#[plugin_help]
pub fn plugin_help() -> RString {
    "Usage: echo [OPTIONS] [STRING]...

Display a line of text.

Options:
        -n    Do not output the trailing newline.

Escape sequences:
        \\n   New line.
        \\t   Horizontal tab.
        \\\\  Backslash.
"
    .into()
}

/// Options for the echo command.
#[derive(Debug, Clone, Copy, Default)]
struct Options {
    /// Whether the output should have a trailing newline.
    /// True by default. `-n` disables it.
    pub trailing_newline: bool,
}

/// Check if an argument is the `-n` flag.
fn is_flag(arg: &str) -> bool {
    arg == "-n"
}

/// Process command line arguments, separating flags from normal arguments.
///
/// # Returns
///
/// - Vector of non-flag arguments.
/// - [`Options`], describing how the arguments should be interpreted.
fn filter_flags(args: Vec<String>) -> (Vec<String>, Options) {
    let mut options = Options::default();
    let mut args_iter = args.iter().peekable();

    // Process `-n` flags until first non-flag is found.
    while let Some(arg) = args_iter.peek() {
        if is_flag(arg) {
            args_iter.next();
            options.trailing_newline = false;
        } else {
            break;
        }
    }

    // Return remaining non-flag arguments.
    (args_iter.cloned().collect(), options)
}

/// Parse escape sequences in a string (simplified: only \n, \t, \\)
fn parse_escapes(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    _ => {
                        // For any other escape sequence, output the backslash and the character
                        result.push('\\');
                        result.push(next);
                    }
                }
            } else {
                // Trailing backslash
                result.push('\\');
            }
        } else {
            result.push(c);
        }
    }

    result
}

#[execute]
pub fn execute(args: RVec<RString>, _last_result: ExecResult) -> ExecResult {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    // Check for help/version flags first
    for arg in &args {
        if arg == "--help" || arg == "-h" {
            return ExecResult {
                code: 255,
                message: rush_internal_plugin_help(),
            };
        }
        if arg == "--version" || arg == "-v" {
            return ExecResult {
                code: 255,
                message: rush_internal_plugin_version(),
            };
        }
    }

    let (args, options) = filter_flags(args);

    let mut output = String::new();
    let mut first = true;

    for arg in args {
        if !first {
            output.push(' ');
        }
        first = false;

        // Always interpret escape sequences
        output.push_str(&parse_escapes(&arg));
    }

    if options.trailing_newline {
        output.push('\n');
    }

    ExecResult::new(0, &output)
}

#[load]
pub fn load() {}
