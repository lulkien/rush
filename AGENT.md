# AGENT.md — Rush Shell

Rush is a learning project: a plugin-based REPL shell written in Rust. It explores shell internals, stable-ABI dynamic loading, and systems programming.

## Workspace Layout

```
rush/                  ← Cargo workspace root
├── rush/              ← Shell binary + core library (the REPL)
├── rush-interface/    ← Stable ABI types shared between shell and plugins
├── rush-macros/       ← Proc macros for plugin authoring
├── rush-plugin/       ← Re-exports for plugin crates (convenience barrel)
└── plugins/
    ├── echo/          ← echo command (with -n, escape sequences)
    ├── pwd/           ← print working directory
    ├── cat/           ← pass-through (placeholder)
    └── rush-prompt/   ← dynamic prompt builder (user@host:dir$)
```

- **Edition**: 2024 across all crates.
- **Channel**: stable Rust; no nightly features.
- **Profile**: `release` uses `opt-level=3`, `lto="fat"`, `strip=true`, `codegen-units=1`.

## Core Architecture

### Startup sequence (`rush/src/lib.rs → start_shell`)

1. `env_logger` initialised (default filter `info`).
2. `user::init_module()` — resolves XDG directories (`~/.local/share/rush`, `~/.config/rush`, `~/.cache/rush`) via `$HOME`, creates them if missing.
3. `env::init_module(&user_dirs)` — builds `EnvRegistry` (a `DashMap<String, Vec<String>>`):
   - Sets `RUSH_DATA_PATH`, `RUSH_CONFIG_PATH`, `RUSH_CACHE_PATH` from XDG dirs + system fallbacks.
   - Loads existing process env vars (colon-split into `Vec<String>`).
4. `shell_builtins::init_module()` — registers built-in commands (currently only `exit`).
5. `plugin::init_module(&env)` — discovers plugins by scanning `*.metadata` files under `$RUSH_DATA_PATH/plugins` and `$RUSH_PLUGIN_PATH` directories. Plugins are registered lazily (not loaded until first execution).
6. `executor::init_module(builtins, plugins)` — wires both registries + a `DashMap`-based command-origin cache into the `Executor`.
7. `InputHandler::new()` — creates a `rustyline::DefaultEditor`.
8. `enter_repl(...)` — main loop.

### REPL loop (`enter_repl`)

1. Load history from `~/.cache/rush/.history`.
2. Generate prompt by executing the `rush-prompt` plugin.
3. Read line via `rustyline`.
4. Lex via `Lexer::new(&line).parse_line()` → returns `CommandPipeList`.
5. Execute via `executor.execute_command_pipe_list(pipe_list)`.
6. Save history on EOF.

### Lexer (`rush/src/lexer/`)

- Wraps `shlex::Shlex` for shell-aware tokenisation (handles quoting, escapes).
- Second pass categorises tokens into `Token::Ident`, `Token::Pipe`, `Token::Semicolon`, `Token::Eof`.
- Handles trailing semicolons on identifiers (splits into `Ident` + `Semicolon`).
- `parse_line()` converts the token stream into a `CommandPipeList`:
  - `CommandPipeList` = `Vec<CommandPipe>` (semicolon-separated).
  - `CommandPipe` = `Vec<Command>` (pipe-separated).
  - `Command` = `{ name: RString, args: RVec<RString> }`.

### Executor (`rush/src/executor.rs`)

- `Executor` holds `BuiltinsRegistry`, `PluginRegistry`, and a `DashMap<String, ExecutionFrom>` cache.
- `ExecutionFrom` enum: `Builtin` | `Plugin` | `NotFound`.
- Cache is populated on first lookup; subsequent lookups skip registry scanning.
- `execute_command_pipe_list`: iterates semicolon-separated pipe groups; each group runs independently.
- `execute_pipe`: **single command** → runs in-process via the plugin/builtin registry. **Multiple commands** (`cmd1 | cmd2 | ...`) → forks N child processes connected by N-1 Unix pipes via `fork()` + `pipe()` + `dup2()`. Each child runs one command, stdin/stdout wired to the adjacent pipes. Parent waits for all children; returns the last child's exit code.
- Output: `println!` from child processes flows through pipes naturally; `print_result()` writes to stdout (code 0) or stderr (code ≠ 0).
- Uses `nix` for safe `fork()`/`pipe()`/`waitpid()` and `libc::dup2` for raw fd wiring. Single `unsafe` block around `fork()` — Rush is single-threaded, each child calls `std::process::exit()`.

### Builtins (`rush/src/shell_builtins/`)

- `BuiltinCommand` trait: `plugin_name`, `print_desc`, `print_help`, `print_version`, `execute`.
- `BuiltinsRegistry` wraps `DashMap<String, Rc<RefCell<Box<dyn BuiltinCommand>>>>`.
- Currently only `exit` is registered. Adding a builtin means:
  1. Create `rush/src/shell_builtins/<name>.rs` implementing `BuiltinCommand`.
  2. Register it in `init_module()`.

### Plugin System (`rush/src/plugin/`)

**ABI layer** (`rush-interface`):
- `Command` struct (stable ABI via `abi_stable`) with C function pointers: `load`, `plugin_name`, `plugin_help`, `plugin_desc`, `plugin_version`, `execute`.
- `ExecResult` struct: `{ code: u8, message: RString }`.
- `CommandRef` is the `RootModule`; base name is `rush_plugin`.

**Proc macros** (`rush-macros`):
- `#[plugin_name]`, `#[plugin_desc]`, `#[plugin_help]`, `#[plugin_version]`, `#[execute]`, `#[load]`.
- Each generates a `#[sabi_extern_fn]` wrapper with a fixed internal name (`rush_internal_plugin_name`, etc.).
- `#[load]` also generates the `#[export_root_module]` FFI entry point (`Command::leak_into_prefix()`).
- The generated `ffi_internal_init_root_module` function is what `abi_stable` calls when loading the `.so`.

**Discovery and loading** (`lazy.rs`, `metadata.rs`):
- Build phase (`build.rs` in each plugin): generates `<plugin_name>.metadata` binary file next to the `.so`.
- Metadata format (all native-endian `u16`):
  - `[0..2]`: total buffer length
  - `[2..4]`: plugin name length
  - `[4..4+name_len]`: plugin name (UTF-8)
  - `[pos..pos+2]`: `.so` filename length
  - `[pos+2..]`: `.so` filename (UTF-8)
- `PluginRegistry` stores `DashMap<String, Rc<RefCell<PluginMetadata>>>`.
- `PluginMetadata`: `{ name, path (to .so), plugin: Option<Rc<CommandRef>> }`.
- On first execution, `PluginMetadata` lazy-loads the `.so` via `abi_stable::library::lib_header_from_path`, calls `init_root_module`, and calls `load()`.

### Environment (`rush/src/env/`)

- `EnvRegistry` is a `DashMap<String, Vec<String>>` (colon-split values).
- Four key variables: `RUSH_DATA_PATH`, `RUSH_CONFIG_PATH`, `RUSH_CACHE_PATH` (set by `default.rs`), and `RUSH_PLUGIN_PATH` (set from OS env).
- Lookup order for data: `~/.local/share/rush` → `/usr/local/share/rush` → `/usr/share/rush`.

### User directories (`rush/src/user/`)

- `UserDirectoryRegistry`: `{ data_dir, config_dir, cache_dir }`.
- Uses `$HOME` + XDG paths; lazy-init via `OnceLock`.

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `abi_stable` 0.11 | Stable ABI for cross-compilation-safe plugin loading |
| `rustyline` 18 | Readline support (history, line editing) |
| `shlex` 1.3 | POSIX shell lexing |
| `dashmap` 6.1 | Concurrent hash maps for registries and caches |
| `nix` 0.31 | Safe `fork()`/`pipe()`/`waitpid()` for pipeline execution |
| `libc` 0.2 | Raw fd operations (`dup2`) for pipe wiring |
| `colored` (plugins) | Terminal color output |
| `gethostname` (rush-prompt) | Hostname for prompt |
| `dirs` (rush-prompt) | Home directory for ~ substitution |
| `env_logger` + `log` | Logging |
| `anyhow` | Error handling |
| `paste` | Macro hygiene in user dir macros |
| `syn` + `quote` (rush-macros) | Proc macro codegen |

## Coding Conventions

- **Rust 2024 edition** throughout.
- **`anyhow::Result`** for all fallible init functions.
- **`DashMap`** for concurrent registries and caches (never `Mutex` or `RwLock` in hot paths).
- **`Rc<RefCell<T>>`** for shared mutable state in registries (single-threaded shell, no `Arc` needed).
- **`ExecResult`** as the universal command return type — even plugins use it.
- **Idiom**: module-level `init_module()` functions return their type via `anyhow::Result<Self>`.
- **No unsafe** in shell core; `abi_stable` handles FFI safety.
- **Logging**: `log` crate macros (`debug!`, `info!`, `warn!`, `error!`); default filter is `info`.

## Building and Running

```bash
# Build everything
cargo build --release

# Run the shell
cargo run --release

# Build only plugins (they output .so + .metadata to target/release)
cargo build --release -p pwd -p echo -p cat -p rush-prompt
```

## Plugin Development Workflow

1. Create a new crate under `plugins/`.
2. Add it to the workspace `Cargo.toml` members.
3. Copy a `build.rs` from an existing plugin (generates `.metadata`).
4. Write the plugin using `rush_plugin::*` macros.
5. Build; the `.so` and `.metadata` land in `target/release/`.
6. Copy both to `~/.local/share/rush/plugins/` (or set `RUSH_PLUGIN_PATH`).
7. The shell discovers it on next start.

**Plugin interface contract**: every plugin must provide:
- `#[plugin_name]` → `fn() -> RString`
- `#[plugin_desc]` → `fn() -> RString`
- `#[plugin_help]` → `fn() -> RString`
- `#[plugin_version]` → `fn() -> RString`
- `#[execute]` → `fn(RVec<RString>, ExecResult) -> ExecResult`
- `#[load]` → `fn()` (called once when `.so` loads)

## Current Limitations (do not attempt, not yet implemented)

- No subshell support.
- No job control / background processes.
- No variable expansion (`$VAR`).
- No tab completion.
- No signal handling (Ctrl+C terminates the shell, Ctrl+Z is unhandled).
- No history search (Ctrl+R).

## Testing

No test suite exists yet. When adding tests:
- Unit tests go in the source file they test (`#[cfg(test)] mod tests { ... }`).
- Integration tests go in `tests/` under the relevant crate.
- Plugin tests may need the `.so` built first.

## File-Specific Notes

- `rush/src/types.rs`: `Command`, `CommandPipe`, `CommandPipeList`, `DashRegistry` trait.
- `rush/src/lexer/token.rs`: `Token` enum; `get_keyword_token` currently a no-op stub (returns `Err` for all input).
- `rush/src/shell_builtins/shared.rs`: exit code constants (`EXIT_SUCCESS = 0`, `EXIT_FAILURE = 1`, `INVALID_ARGS = 2`).
- `plugins/*/build.rs`: all follow the same pattern — write `.metadata` to `target/release/` (3 parents above `OUT_DIR`).
- `rush/Cargo.toml`: the binary crate depends on `rush-interface` and `rush-macros`; plugins depend on `rush-plugin` (which re-exports both).
