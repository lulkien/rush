use std::{cell::RefCell, rc::Rc};

use anyhow::anyhow;
use dashmap::{DashMap, try_result::TryResult};
use rush_interface::CommandResult;

pub use crate::plugin::metadata::PluginMetadata;
use crate::{
    var::VarStore,
    types::{Command, DashRegistry},
};

mod lazy;

mod metadata;

type RegistryTypeRaw = PluginMetadata;
type RegistryType = Rc<RefCell<RegistryTypeRaw>>;

#[derive(Default)]
pub struct PluginRegistry(pub DashMap<String, RegistryType>);

impl DashRegistry<RegistryTypeRaw> for PluginRegistry {
    fn register(&self, name: &str, metadata: RegistryType) {
        self.0.insert(name.to_string(), metadata);
    }

    fn unregister(&self, name: &str) {
        self.0.remove(name);
    }

    fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }

    fn get(&self, name: &str) -> anyhow::Result<RegistryType> {
        match self.0.try_get(name) {
            TryResult::Absent => Err(anyhow!("Plugin not existed: {name}")),
            TryResult::Locked => Err(anyhow!("Registry locked. PLEASE CHECK!!!")),
            TryResult::Present(metadata) => Ok(metadata.clone()),
        }
    }
}

impl PluginRegistry {
    pub fn execute(&self, command: Command) -> CommandResult {
        let plugin_metadata = match self.get(&command.name) {
            Ok(metadata) => metadata,
            Err(e) => return CommandResult::new(1, format!("{e}").as_str()),
        };

        let mut metadata_ref = plugin_metadata.as_ref().borrow_mut();

        if !metadata_ref.is_loaded() {
            let plugin = match lazy::load_plugin(&metadata_ref.path) {
                Some(p) => p,
                None => {
                    return CommandResult::new(
                        1,
                        format!("{}: plugin failed to load", command.name).as_str(),
                    );
                }
            };

            metadata_ref.plugin = Some(plugin);
        }

        // Drop mutable borrow, we done here
        drop(metadata_ref);

        // Convert Vec<String> to FFI types for the plugin ABI.
        let ffi_args: abi_stable::std_types::RVec<abi_stable::std_types::RString> =
            command.args.iter().map(|s| s.as_str().into()).collect();

        match plugin_metadata.as_ref().borrow().plugin.as_ref() {
            Some(module) => module.execute()(ffi_args),
            None => CommandResult::new(1, "plugin unloaded unexpectedly"),
        }
    }

    pub fn names(&self) -> Vec<String> {
        self.0.iter().map(|e| e.key().clone()).collect()
    }
}

pub fn init_module(vars: &VarStore) -> anyhow::Result<PluginRegistry> {
    let mut plugin_registry = PluginRegistry::default();

    lazy::discover(&mut plugin_registry, vars)?;

    Ok(plugin_registry)
}
