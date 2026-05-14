use std::path::Path;

use rustyline::{DefaultEditor, error::ReadlineError};

pub struct InputHandler {
    editor: DefaultEditor,
}

impl InputHandler {
    pub fn new() -> anyhow::Result<Self> {
        let editor = DefaultEditor::new()?;
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
