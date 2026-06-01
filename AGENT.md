# AGENT.md — Rush Shell

Rush is a POSIX-compatible shell written in Rust (edition 2024, stable channel). It has a plugin system with stable ABI, pipeline execution, file redirects, and variable expansion.

## Workspace Layout

```
rush-core/          Shell engine
  types.rs          AST: Command, Pipeline, Program, Redirect, AndOrList
  lexer/            logos-based tokenizer with QuoteKind
  parser/           recursive-descent POSIX parser
  executor.rs       command dispatch, pipes, redirects, PATH lookup
  var.rs            VarStore: $VAR/$?/$$ expansion, export tracking
  plugin/           PluginRegistry, ABI .so loading, metadata discovery
  shell_builtins/   cd, export, unset, history-search
  tests/            6 pipe integration tests

rush/               CLI + REPL (depends on rush-core)
  main.rs           clap CLI: -c, script, interactive
  lib.rs            start_shell(), enter_repl(), run_script()
  input.rs          rustyline: fuzzy completion, PATH completer, Ctrl+Backspace

rush-interface/     FFI types: CommandResult { code: i32, message: RString }, Module
rush-macros/        proc macros: #[execute], #[plugin_name], #[load], etc.
rush-plugin/        re-exports rush-interface + rush-macros
plugins/            echo, exit, pwd, rush-prompt
```

## Execution Flow

```
input string
  → lexer::Lexer::tokenize() → Vec<Token>
  → parser::parse() → Program AST
  → preprocess() — extract VAR=value assignments
  → expand_command() — replace $VAR, ${VAR}, $?, $$
  → executor.execute_pipeline() — per-command dispatch:
       resolve(name) → Builtin | Plugin | External(PathBuf) | NotFound
       builtin → self.builtin_reg.execute(command, &vars)
       plugin  → self.plugin_reg.execute(command) (with FFI conversion)
       external → fork + execve (with env from VarStore)
       pipe    → fork N children, dup2 pipes, waitpid
       redirect → apply_redirects before/after execution
```

## Key Conventions

- **Rust 2024**, stable channel, zero clippy warnings
- **Commits**: conventional format, scoped (`feat(executor):`, `fix(lexer):`)
- **Tests**: 48 total (42 unit + 6 pipe), run with `--test-threads=1` (stdout capture)
- **nix 0.31** for syscalls (fork, pipe, dup2, signal, waitpid)
- **logos 0.15** for lexing — raw strings need `r#"..."#` when regex contains double-quote
- **DashMap** for concurrent registries and caches (not Mutex/RwLock)
- **Rc<RefCell<T>>** for shared mutable state in registries

## Plugin System

- Plugins are `.so` files (cdylib) with a companion `.metadata` binary file
- Discovered from `RUSH_DATA_PATH/plugins/` and `RUSH_PLUGIN_PATH`
- Loaded lazily on first execution via `abi_stable`
- ABI types: `CommandResult` (code: i32, message: RString)
- Macros: `#[plugin_name]`, `#[plugin_desc]`, `#[plugin_help]`, `#[plugin_version]`, `#[execute]`, `#[load]`
- Registration counts only new plugins (skip duplicates via `contains()` check)

### FFI Boundary

The only place `abi_stable::RString`/`RVec` are used is `plugin/mod.rs`:
```rust
let ffi_args: RVec<RString> = command.args.iter().map(|s| s.as_str().into()).collect();
module.execute()(ffi_args)
```

All internal types use native `String`/`Vec<String>`.

## Variable Store

- All values are `Vec<String>` internally
- Single values as `["val"]`, PATH-like split/joined on `:`
- `$?` stored as special `?` key, updated by `set_exit_code()` after each command
- Expansion happens per-command in `execute_string()` so `$?` reflects prior result
- Single-quoted args marked with `\x01` prefix in parser, skipped during expansion
- All vars exported by default; `export` marks specific vars (filtered in `build_env_array`)

## Pipe Implementation

- `execute_pipe_forked`: creates N-1 pipes with CLOEXEC, forks N children
- Each child: dup2 stdin/stdout to pipe ends, then exec/lookup
- `dup2` target uses `OwnedFd::from_raw_fd(STDIN_FILENO)` + `std::mem::forget`
- Parent waits for all children, returns last exit code

## Redirects

- `Redirect { op, src_fd: Option<i32>, target: String }`
- Parser detects bare-number fd prefixes (both before name and in args)
- External/pipe: `apply_redirects_for_child` before exec
- Plugin/builtin: `save_and_apply_redirects` → execute → `restore_redirect_fds`
- `print_result` called before restore so output goes to redirected fd

## Signal Handling

- Parent ignores SIGINT via `SigAction::SigIgn`
- Children reset to SIG_DFL after fork
- Ctrl+C kills only foreground children, not the shell

## Tab Completion

- First word: builtins + plugins + PATH executables (lazy scan, cached in RefCell)
- Otherwise: filename completion via FilenameCompleter
- Ctrl+Backspace: backward-kill-word (ESC+DEL and ESC+^H sequences)

## Testing

```bash
cargo test -- --test-threads=1   # 48 tests (42 unit + 6 pipe)
```

Pipe tests manipulate stdout via `dup2` — must run single-threaded.
