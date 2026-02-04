mod lazy;
mod registry;

use std::{path::PathBuf, sync::Arc};

use rush_interface::CommandRef;

pub use lazy::{get_plugin};

#[allow(unused)]
struct PluginMetadata {
    name: String,
    path: PathBuf,
    plugin: Option<Arc<CommandRef>>,
}

impl PluginMetadata {
    pub fn new(name: &str, path: PathBuf) -> Self {
        Self {
            name: name.to_owned(),
            path,
            plugin: None,
        }
    }
    pub fn is_loaded(&self) -> bool {
        self.plugin.is_some()
    }
}

pub fn init_module() -> anyhow::Result<()> {
    lazy::discover_plugins()?;

    Ok(())
}
