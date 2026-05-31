use std::{cell::RefCell, rc::Rc};

use abi_stable::std_types::{RString, RVec};
use anyhow::anyhow;
use dashmap::{DashMap, try_result::TryResult};
use log::warn;
use rush_interface::ExecResult;

use crate::types::{Command, DashRegistry};

mod history_search;

#[allow(unused)]
pub trait BuiltinCommand: Send + Sync {
    fn plugin_name(&self) -> RString;
    fn print_desc(&self);
    fn print_help(&self);
    fn print_version(&self);
    fn execute(&self, args: RVec<RString>) -> ExecResult;
}

type RegistryTypeRaw = Box<dyn BuiltinCommand>;
type RegistryType = Rc<RefCell<RegistryTypeRaw>>;

#[derive(Default)]
pub struct BuiltinsRegistry(DashMap<String, RegistryType>);

impl DashRegistry<RegistryTypeRaw> for BuiltinsRegistry {
    fn register(&self, name: &str, builtin: RegistryType) {
        if self.contains(name) {
            warn!("[OVERRIDE WARNING] Shell builtin has been registered.");
        }
        self.0.insert(name.to_string(), builtin);
    }

    fn unregister(&self, name: &str) {
        self.0.remove(name);
    }

    fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    fn get(&self, name: &str) -> anyhow::Result<RegistryType> {
        match self.0.try_get_mut(name) {
            TryResult::Absent => Err(anyhow!("Builtin not existed: {name}")),
            TryResult::Locked => Err(anyhow!("Registry locked. PLEASE CHECK!!!")),
            TryResult::Present(command) => Ok(command.clone()),
        }
    }
}

impl BuiltinsRegistry {
    pub fn execute(&self, command: Command) -> ExecResult {
        match self.get(&command.name) {
            Ok(command_entry) => command_entry
                .as_ref()
                .borrow()
                .execute(command.args),
            Err(e) => ExecResult::new(1, format!("{e}").as_str()),
        }
    }

    pub fn names(&self) -> Vec<String> {
        self.0.iter().map(|e| e.key().clone()).collect()
    }
}

pub fn init_module() -> anyhow::Result<BuiltinsRegistry> {
    let builtin_registry = BuiltinsRegistry::default();

    builtin_registry.register(
        "history-search",
        Rc::new(RefCell::new(Box::new(history_search::Command {}))),
    );

    Ok(builtin_registry)
}
