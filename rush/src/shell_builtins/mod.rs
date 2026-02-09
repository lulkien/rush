use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock, RwLockReadGuard},
};

use abi_stable::std_types::{RString, RVec};
use anyhow::bail;
use rush_interface::ExecResult;

mod exit;
mod plugin_desc;
mod plugin_help;
mod plugin_version;
mod shared;

static BUILTINS_REGISTRY: OnceLock<RwLock<BuiltinsRegistry>> = OnceLock::new();

#[allow(unused)]
pub trait BuiltinCommand: Send + Sync {
    fn print_help(&self);
    fn print_version(&self);
    fn execute(&self, args: RVec<RString>) -> ExecResult;
}

#[derive(Default)]
pub struct BuiltinsRegistry {
    commands: HashMap<String, Arc<Box<dyn BuiltinCommand>>>,
}

impl BuiltinsRegistry {
    fn insert_command(
        &mut self,
        name: &str,
        command: Arc<Box<dyn BuiltinCommand>>,
    ) -> anyhow::Result<()> {
        if self.contains(name) {
            bail!("{}: built-in command already exists", name);
        }
        self.commands.insert(name.to_string(), command);
        Ok(())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn execute(&self, builtin_name: &str, args: RVec<RString>) -> ExecResult {
        if let Some(command) = self.commands.get(builtin_name) {
            command.execute(args)
        } else {
            ExecResult::new(1, &format!("{}: built-in command not found", builtin_name))
        }
    }
}

pub fn builtins_registry() -> anyhow::Result<RwLockReadGuard<'static, BuiltinsRegistry>> {
    BUILTINS_REGISTRY
        .get_or_init(|| RwLock::new(BuiltinsRegistry::default()))
        .read()
        .map_err(|e| anyhow::anyhow!("BUILTINS_REGISTRY read lock poisoned: {e}"))
}

pub fn init_module() -> anyhow::Result<()> {
    let mut builtins = BUILTINS_REGISTRY
        .get_or_init(|| RwLock::new(BuiltinsRegistry::default()))
        .write()
        .map_err(|e| anyhow::anyhow!("BUILTINS_REGISTRY write lock poisoned: {e}"))?;

    builtins.insert_command("exit", Arc::new(Box::new(exit::Command {})))?;
    builtins.insert_command("plugin-desc", Arc::new(Box::new(plugin_desc::Command {})))?;
    builtins.insert_command("plugin-help", Arc::new(Box::new(plugin_help::Command {})))?;
    builtins.insert_command(
        "plugin-version",
        Arc::new(Box::new(plugin_version::Command {})),
    )?;

    Ok(())
}
