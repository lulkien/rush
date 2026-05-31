use std::cell::RefCell;
use std::path::Path;

use rustyline::{
    Editor,
    completion::{Completer, FilenameCompleter},
    error::ReadlineError,
    hint::{Hinter, HistoryHinter},
    highlight::Highlighter,
    validate::Validator,
    Helper, Context,
};

/// Bundles all rustyline extension traits for our shell.
struct InputHelper {
    hinter: HistoryHinter,
    commands: Vec<String>,
    /// Lazily-populated cache of PATH executables.
    path_commands: RefCell<Option<Vec<String>>>,
}

impl InputHelper {
    fn new() -> Self {
        Self {
            hinter: HistoryHinter {},
            commands: Vec::new(),
            path_commands: RefCell::new(None),
        }
    }

    fn set_commands(&mut self, cmds: Vec<String>) {
        self.commands = cmds;
        self.commands.sort();
        self.commands.dedup();
    }
}

// ── Completer: commands for first word, filenames otherwise ──────────

impl Completer for InputHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        // If cursor is in the first word, complete commands.
        if is_first_word(line, pos) {
            let word = &line[..pos];
            let start = word.rfind(|c: char| c.is_whitespace()).map_or(0, |i| i + 1);
            let prefix = &line[start..pos];

            let mut matches: Vec<String> = self
                .commands
                .iter()
                .filter(|c| c.starts_with(prefix))
                .cloned()
                .collect();

            // Lazily scan PATH for external executables (once).
            let mut path_cache = self.path_commands.borrow_mut();
            if path_cache.is_none() {
                *path_cache = Some(scan_path_executables());
            }
            if let Some(ref path_cmds) = *path_cache {
                for c in path_cmds {
                    if c.starts_with(prefix) && !self.commands.contains(c) {
                        matches.push(c.clone());
                    }
                }
            }

            matches.sort();
            matches.dedup();
            if !matches.is_empty() {
                return Ok((start, matches));
            }
        }

        // Fall back to filename completion.
        let fc = FilenameCompleter::new();
        fc.complete(line, pos, _ctx)
            .map(|(start, pairs)| (start, pairs.into_iter().map(|p| p.replacement).collect()))
    }
}

/// True if the cursor is at the first whitespace-delimited word.
fn is_first_word(line: &str, pos: usize) -> bool {
    let before = &line[..pos];
    !before.contains(|c: char| c.is_whitespace() && c != ' ')
        || before.trim_start().is_empty()
}

/// Scan PATH once and return all executable names.
fn scan_path_executables() -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let Ok(path_var) = std::env::var("PATH") else {
        return Vec::new();
    };
    for dir in path_var.split(':') {
        if dir.is_empty() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if seen.contains(&name) {
                continue;
            }
            if is_executable(&entry.path()) {
                seen.insert(name.clone());
            }
        }
    }
    let mut names: Vec<String> = seen.into_iter().collect();
    names.sort();
    names
}

/// Check whether a file is a regular file with at least one executable bit.
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

// ── other traits ─────────────────────────────────────────────────────

impl Hinter for InputHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for InputHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        if hint.is_empty() {
            return std::borrow::Cow::Borrowed("");
        }
        // ANSI faint (dim) + bright black for ghost text.
        std::borrow::Cow::Owned(format!("\x1b[2m\x1b[90m{}\x1b[0m", hint))
    }
}

impl Validator for InputHelper {}

impl Helper for InputHelper {}

pub struct InputHandler {
    editor: Editor<InputHelper, rustyline::history::FileHistory>,
}

impl InputHandler {
    pub fn new() -> anyhow::Result<Self> {
        let config = rustyline::Config::builder()
            .history_ignore_dups(true)?
            .completion_type(rustyline::config::CompletionType::Fuzzy)
            .build();
        let helper = InputHelper::new();
        let mut editor: Editor<InputHelper, rustyline::history::FileHistory> =
            Editor::with_config(config)?;
        editor.set_helper(Some(helper));
        Ok(Self { editor })
    }

    /// Set the available command names for tab completion.
    pub fn set_commands(&mut self, cmds: Vec<String>) {
        if let Some(h) = self.editor.helper_mut() {
            h.set_commands(cmds);
        }
    }

    pub fn load_history<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        self.editor.load_history(&path)?;
        Ok(())
    }

    pub fn readline(&mut self, prompt: &str) -> Result<String, ReadlineError> {
        self.editor.readline(prompt)
    }

    pub fn add_history(&mut self, entry: &str) -> anyhow::Result<()> {
        self.editor.add_history_entry(entry)?;
        Ok(())
    }

    /// Launch an interactive fuzzy history search using the skim TUI.
    /// Returns the selected entry, or None if cancelled.
    pub fn history_search(&self) -> Option<String> {
        use rustyline::history::History;
        use skim::prelude::*;

        // Collect history entries, newest first, deduplicated.
        let h = self.editor.history();
        let mut seen = std::collections::HashSet::new();
        let mut history: Vec<String> = Vec::new();
        for i in (0..h.len()).rev() {
            if let Ok(Some(result)) =
                h.get(i, rustyline::history::SearchDirection::Forward)
                && seen.insert(result.entry.to_string())
            {
                history.push(result.entry.to_string());
            }
        }

        if history.is_empty() {
            return None;
        }

        let options = SkimOptionsBuilder::default()
            .prompt("history> ".to_string())
            .reverse(true)
            .height("50%".to_string())
            .build()
            .unwrap();

        let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
        struct HistoryItem(String);
        impl SkimItem for HistoryItem {
            fn text(&self) -> std::borrow::Cow<'_, str> {
                std::borrow::Cow::Borrowed(&self.0)
            }
        }
        let items: Vec<std::sync::Arc<dyn SkimItem>> = history
            .into_iter()
            .map(|s| -> std::sync::Arc<dyn SkimItem> {
                std::sync::Arc::new(HistoryItem(s))
            })
            .collect();
        let _ = tx.send(items);
        drop(tx);

        Skim::run_with(options, Some(rx))
            .ok()
            .and_then(|out| out.selected_items.first().map(|item| item.output().to_string()))
    }

    pub fn save_history<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        self.editor.save_history(&path)?;
        Ok(())
    }
}
