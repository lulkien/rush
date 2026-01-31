use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use crate::plugin::PluginMetadata;

#[derive(Default)]
pub struct PluginRegistry {
    registry: HashMap<String, PluginMetadata>,
}

static PLUGIN_REGISTRY: OnceLock<RwLock<PluginRegistry>> = OnceLock::new();

#[allow(unused)]
impl PluginRegistry {
    pub fn add(&mut self, name: &str, plugin: PluginMetadata) {
        let _ = self.registry.insert(name.to_owned(), plugin);
    }

    pub fn remove(&mut self, name: &str) -> Option<PluginMetadata> {
        self.registry.remove(name)
    }

    pub fn borrow_mut(&mut self, name: &str) -> Option<&mut PluginMetadata> {
        self.registry.get_mut(name)
    }

    pub fn borrow_ref(&self, name: &str) -> Option<&PluginMetadata> {
        self.registry.get(name)
    }
}

fn plugin_registry() -> &'static RwLock<PluginRegistry> {
    PLUGIN_REGISTRY.get_or_init(|| RwLock::new(PluginRegistry::default()))
}

pub(super) fn read_plugin_registry() -> anyhow::Result<RwLockReadGuard<'static, PluginRegistry>> {
    plugin_registry()
        .read()
        .map_err(|_| anyhow::anyhow!("PLUGIN_REGISTRY read lock poisoned"))
}

pub(super) fn write_plugin_registry() -> anyhow::Result<RwLockWriteGuard<'static, PluginRegistry>> {
    plugin_registry()
        .write()
        .map_err(|_| anyhow::anyhow!("PLUGIN_REGISTRY read lock poisoned"))
}
