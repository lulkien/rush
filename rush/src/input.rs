use std::{
    io::{Error, ErrorKind},
    path::Path,
    sync::{OnceLock, RwLock, RwLockWriteGuard},
};

use rustyline::{DefaultEditor, error::ReadlineError};

static READLINE: OnceLock<RwLock<DefaultEditor>> = OnceLock::new();

pub fn init_module() -> anyhow::Result<()> {
    get_readline();

    Ok(())
}

fn get_readline() -> &'static RwLock<DefaultEditor> {
    READLINE.get_or_init(|| {
        RwLock::new(DefaultEditor::new().expect("Failed to create readline editor"))
    })
}

fn write_readline() -> anyhow::Result<RwLockWriteGuard<'static, DefaultEditor>> {
    get_readline()
        .write()
        .map_err(|_| anyhow::anyhow!("Readline write lock poisoned"))
}

pub fn load_history(path: &std::path::Path) -> anyhow::Result<()> {
    write_readline()?.load_history(path)?;
    Ok(())
}

pub fn readline(prompt: &str) -> Result<String, ReadlineError> {
    write_readline()
        .map_err(|_| ReadlineError::Io(Error::from(ErrorKind::Deadlock)))?
        .readline(prompt)
}

pub fn add_history(entry: &str) -> anyhow::Result<()> {
    write_readline()?.add_history_entry(entry)?;
    Ok(())
}

pub fn save_history<P: AsRef<Path>>(path: P) -> anyhow::Result<()> {
    write_readline()?.save_history(&path)?;
    Ok(())
}
