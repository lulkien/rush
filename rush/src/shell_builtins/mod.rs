use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock, RwLockReadGuard},
};

use abi_stable::std_types::{RString, RVec};
use anyhow::bail;
use rush_interface::ExecResult;

mod plugin_desc;
mod plugin_help;
mod plugin_version;

static BUILTINS_REGISTRY: OnceLock<RwLock<BuiltinsRegistry>> = OnceLock::new();

#[allow(unused)]
pub trait BuiltinCommand: Send + Sync {
    fn help(&self) -> &str;
    fn desc(&self) -> &str;
    fn version(&self) -> &str;
    fn exec(&self, args: &str) -> ExecResult;
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

    fn get_command(&self, name: &str) -> Option<Arc<Box<dyn BuiltinCommand>>> {
        self.commands.get(name).cloned()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.commands.contains_key(name)
    }

    pub fn execute(&self, builtin_name: &str, mut args: RVec<RString>) -> ExecResult {
        if args.is_empty() {
            return ExecResult::new(1, &format!("{}: no argument", builtin_name));
        }

        let plugin_name = args.remove(0).to_string();

        if let Some(command) = self.commands.get(builtin_name) {
            command.exec(&plugin_name)
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

    builtins.insert_command("plugin-desc", Arc::new(Box::new(plugin_desc::Command {})))?;
    builtins.insert_command("plugin-help", Arc::new(Box::new(plugin_help::Command {})))?;
    builtins.insert_command(
        "plugin-version",
        Arc::new(Box::new(plugin_version::Command {})),
    )?;

    Ok(())
}
