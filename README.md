# Rush Shell

A POSIX-compatible shell written in Rust, with plugin support and a clean internal architecture.

## Features

- **Interactive REPL** with fuzzy tab completion, ghost-text hints, and history search (Ctrl+R)
- **Pipeline execution** — `cmd1 | cmd2 | cmd3` with proper fork/pipe/dup2
- **File redirects** — `>`, `>>`, `<`, `2>`
- **External commands** — runs any binary from `$PATH`
- **Variables** — `VAR=value`, `$VAR`, `${VAR}`, `$?`, `$$`
- **Builtins** — `cd`, `export`, `unset`, `history-search`
- **Plugin system** — stable-ABI `.so` plugins with lazy loading
- **Signal handling** — Ctrl+C kills foreground children, not the shell

## Quick Start

```bash
cargo build --release
cargo run --release
```

```sh
$ echo hello > /tmp/out && cat /tmp/out
hello
$ ls -la | wc -l
42
$ cd /tmp && pwd
/tmp
$ TEST=world; echo $TEST
world
$ false; echo $?
1
```

## Architecture

```
rush-core/          Shell engine (types, lexer, parser, executor, variables, plugins)
rush/               CLI entry point + REPL (rustyline)
rush-interface/     Plugin ABI (CommandResult, Module)
rush-macros/        Proc macros for plugins (#[execute], #[plugin_name], ...)
rush-plugin/        Re-exports for plugin crates
plugins/            echo, exit, pwd, rush-prompt
```

### rush-core modules

| Module | Purpose |
|--------|---------|
| `types` | AST nodes: Command, Pipeline, Program, Redirect |
| `lexer` | logos-based tokenizer with quote tracking |
| `parser` | Recursive-descent POSIX parser |
| `executor` | Command dispatch, pipes, redirects, PATH lookup |
| `var` | Variable store, expansion, export tracking |
| `plugin` | Plugin discovery, lazy `.so` loading |
| `shell_builtins` | cd, export, unset, history-search |

### Execution flow

1. `lexer::Lexer::tokenize()` → `Vec<Token>`
2. `parser::parse()` → `Program` AST
3. `preprocess()` — extract `VAR=value` assignments
4. `expand_command()` — replace `$VAR` references
5. `executor.execute_pipeline()` — dispatch to builtin/plugin/external

## Plugin System

Plugins are `.so` files with a `.metadata` sidecar, discovered from `RUSH_DATA_PATH/plugins/` and `RUSH_PLUGIN_PATH`.

### Creating a plugin

```rust
use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;

#[plugin_name]
pub fn plugin_name() -> RString { "mycmd".into() }

#[execute]
pub fn execute(args: RVec<RString>) -> CommandResult {
    CommandResult::new(0, "hello from mycmd")
}

#[load]
pub fn load() {}
```

Build and deploy:

```bash
cargo build --release -p mycmd
cp target/release/libmycmd.so target/release/mycmd.metadata ~/.local/share/rush/plugins/
```

## Building

```bash
cargo build --release    # everything
cargo test -- --test-threads=1   # 48 tests
```

## License

MIT
