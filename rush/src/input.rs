use std::path::Path;

use rustyline::{
    Editor,
    completion::Completer,
    error::ReadlineError,
    hint::{Hinter, HistoryHinter},
    highlight::Highlighter,
    validate::Validator,
    Helper,
};

/// Bundles all rustyline extension traits for our shell.
struct InputHelper {
    hinter: HistoryHinter,
}

impl InputHelper {
    fn new() -> Self {
        Self {
            hinter: HistoryHinter {},
        }
    }
}

impl Completer for InputHelper {
    type Candidate = String;
}

impl Hinter for InputHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
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
