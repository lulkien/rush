# Rush Shell

A modern, plugin-based shell in Rust.

## Project Overview

Rush is a learning project to explore shell internals, plugin architecture, and systems programming in Rust. It implements a REPL shell with readline support, a stable-ABI plugin system, and pipeline execution.

## Features

- **REPL shell** with readline support ([`rustyline`](https://crates.io/crates/rustyline))
- **Plugin system** with stable ABI ([`abi_stable`](https://crates.io/crates/abi_stable))
- **Pipeline execution** (`|`) - commands connected by pipe execute sequentially, output flows between them
- **Command chaining** (`;`) - semicolon-separated commands execute independently
- **Built-in commands** - [`exit`](rush/src/shell_builtins/exit.rs:1) command implemented
- **Dynamic prompt** - colorized prompt with user, hostname, directory via [`rush-prompt`](plugins/rush-prompt/src/lib.rs:1)
- **XDG-compliant** user directory management ([`user`](rush/src/user/mod.rs:1))

## Architecture

### Core Components

- [`rush/`](rush/src/) - Main shell implementation
  - [`executor`](rush/src/executor.rs:1) - Command execution dispatcher with caching
  - [`input`](rush/src/input.rs:1) - Readline-based input handling
  - [`lexer`](rush/src/lexer/) - Tokenization with [`shlex`](https://crates.io/crates/shlex)
  - [`plugin`](rush/src/plugin/) - Plugin loading and registry with lazy loading
  - [`shell_builtins`](rush/src/shell_builtins/) - Built-in commands registry
  - [`user`](rush/src/user/) - XDG-compliant user directory management
  - [`env`](rush/src/env/) - Environment registry

### Interfaces

- [`rush-interface`](rush-interface/src/lib.rs:1) - Plugin ABI interface using `abi_stable`
- [`rush-plugin`](rush-plugin/src/lib.rs:1) - Plugin re-exports and macros
- [`rush-macros`](rush-macros/src/lib.rs:1) - Proc macros for plugin development

### Plugins

- [`pwd`](plugins/pwd/src/lib.rs:1) - Print working directory
- [`echo`](plugins/echo/src/lib.rs:1) - Echo arguments to stdout
- [`cat`](plugins/cat/src/lib.rs:1) - Pass through input (placeholder for file reading)
- [`rush-prompt`](plugins/rush-prompt/src/lib.rs:1) - Dynamic prompt builder

## Plugin System

Plugins are compiled as `cdylib` with stable ABI exports.

### Plugin ABI Interface

[`Command`](rush-interface/src/lib.rs:13) struct defines plugin interface:

```rust
pub struct Command {
    pub load: extern "C" fn(),
    pub plugin_name: extern "C" fn() -> RString,
    pub print_help: extern "C" fn(),
    pub print_desc: extern "C" fn(),
    pub print_version: extern "C" fn(),
    pub execute: extern "C" fn(RVec<RString>, ExecResult) -> ExecResult,
}
```

### Build Process

1. Plugin source compiled to `.so` (e.g., `libpwd.so`)
2. [`build.rs`](plugins/pwd/build.rs:1) generates `.metadata` file containing:
   - Plugin name length (2 bytes)
   - Plugin name (variable)
   - SO filename length (2 bytes)
   - SO filename (variable)
3. Rush loads plugins by reading `.metadata` files from `RUSH_DATA_PATH` directories

### Plugin Development

Use macros from [`rush-macros`](rush-macros/src/lib.rs:1):

- [`#[plugin_name]`](rush-macros/src/lib.rs:37) - Plugin identifier
- [`#[execute]`](rush-macros/src/lib.rs:101) - Command handler
- [`#[print_help]`](rush-macros/src/lib.rs:69) - Help text
- [`#[print_desc]`](rush-macros/src/lib.rs:53) - Description
- [`#[print_version]`](rush-macros/src/lib.rs:85) - Version
- [`#[load]`](rush-macros/src/lib.rs:6) - Initialization

Example plugin:

```rust
use abi_stable::std_types::{RString, RVec};
use rush_plugin::*;

#[plugin_name]
pub fn plugin_name() -> RString {
    env!("CARGO_PKG_NAME").into()
}

#[execute]
pub fn execute(args: RVec<RString>, _last_result: ExecResult) -> ExecResult {
    let message: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let output = message.join(" ");
    ExecResult::new(0, &output)
}

#[load]
pub fn load() {}
```

### Plugin Loading Flow

```mermaid
flowchart TD
    A[Shell Start] --> B[init_module]
    B --> C[Discover from RUSH_DATA_PATH]
    C --> D[Read .metadata files]
    D --> E[Register PluginMetadata]
    F[Command Execution] --> G{Plugin Loaded?}
    G -->|No| H[Load Plugin SO]
    H --> I[Call load() fn]
    G -->|Yes| J[Execute Command]
    I --> J
```

## Environment Variables

| Variable | Description | Default |
| -------- | ----------- | ------- |
| `RUSH_DATA_PATH` | Data directory (plugins, resources) | `$XDG_DATA_HOME/rush:/usr/local/share/rush:/usr/share/rush` |
| `RUSH_CONFIG_PATH` | Configuration directory | `$XDG_CONFIG_HOME/rush:/etc/rush` |
| `RUSH_CACHE_PATH` | Cache directory | `$XDG_CACHE_HOME/rush` |
| `RUSH_PLUGIN_PATH` | Additional plugin directories (colon-separated) | - |

## Building & Usage

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run --release
```

### Examples

```bash
# Simple command
pwd

# Pipeline
ls -l | grep .rs

# Command chaining
echo hello; echo world

# Exit shell
exit
```

## Current Limitations

- No file I/O in plugins (cat plugin just passes through input)
- No subshell support
- No job control
- No variable expansion
- No history search
- No tab completion
- No signal handling (Ctrl+C, Ctrl+Z)
- No exit status propagation between commands in pipeline
