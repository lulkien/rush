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
}

impl InputHelper {
    fn new() -> Self {
        Self {
            hinter: HistoryHinter {},
            commands: Vec::new(),
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
            let matches: Vec<String> = self
                .commands
                .iter()
                .filter(|c| c.starts_with(prefix))
                .cloned()
                .collect();
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

// ── other traits ─────────────────────────────────────────────────────

impl Hinter for InputHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl Highlighter for InputHelper {}

impl Validator for InputHelper {}

impl Helper for InputHelper {}

pub struct InputHandler {
    editor: Editor<InputHelper, rustyline::history::FileHistory>,
}

impl InputHandler {
    pub fn new() -> anyhow::Result<Self> {
        let config = rustyline::Config::builder()
            .history_ignore_dups(true)?
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

    pub fn save_history<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        self.editor.save_history(&path)?;
        Ok(())
    }
}
