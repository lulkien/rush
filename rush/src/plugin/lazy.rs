#![allow(unused)]

use std::{fs, path::Path, sync::Arc};

use anyhow::{Context, ensure};
use log::{debug, info};
use rush_interface::CommandRef;

use crate::{
    env::read_rush_data_dirs,
    plugin::{
        PluginMetadata,
        registry::{read_plugin_registry, write_plugin_registry},
    },
};

pub fn get_plugin(name: &str) -> anyhow::Result<Arc<CommandRef>> {
    // Try optimistic read first
    {
        let registry_reader = read_plugin_registry()?;
        if let Some(metadata) = registry_reader.borrow_ref(name)
            && metadata.is_loaded()
        {
            return Ok(metadata.plugin.clone().unwrap());
        }
    }

    // Get write lock
    let mut registry_writer = write_plugin_registry()?;

    let metadata_mut = registry_writer
        .borrow_mut(name)
        .ok_or_else(|| anyhow::anyhow!("{}: command not found", name))?;

    let plugin = load_plugin(&metadata_mut.path);
    metadata_mut.plugin = plugin.clone();

    plugin.ok_or_else(|| anyhow::anyhow!("{}: plugin failed to load", name))
}

pub fn reload_plugin(name: &str) -> anyhow::Result<Arc<CommandRef>> {
    let mut registry_writer = write_plugin_registry()?;

    let metadata_mut = registry_writer
        .borrow_mut(name)
        .ok_or_else(|| anyhow::anyhow!("{}: command not found", name))?;

    metadata_mut.plugin = None;

    let plugin = load_plugin(&metadata_mut.path);
    metadata_mut.plugin = plugin.clone();

    plugin.ok_or_else(|| anyhow::anyhow!("{}: plugin failed to load", name))
}

pub(super) fn discover_plugins() -> anyhow::Result<()> {
    let mut registered_count = 0;

    read_rush_data_dirs()?.iter().for_each(|path| {
        let plugin_path = path.join("plugins");
        registered_count += discover_plugins_from_dir(plugin_path).unwrap_or_default();
    });

    info!("Registered {} plugin(s)", registered_count);

    Ok(())
}

fn discover_plugins_from_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<usize> {
    let path = path.as_ref();
    let mut registered_count = 0;

    ensure!(path.exists(), format!("{} not found", path.display()));
    ensure!(
        path.is_dir(),
        format!("{} is not a directory", path.display())
    );

    debug!("Load plugin from: {}", path.display());

    let entries = fs::read_dir(path)
        .with_context(|| format!("Failed to read directory: {}", path.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("Failed to read entry in {}", path.display()))?;
        let path = entry.path();

        if let Some(plugin_name) = is_plugin_file(&path) {
            debug!("Registered plugin path: {}", path.display());
            registered_count += 1;
            write_plugin_registry()?.add(&plugin_name, PluginMetadata::new(&plugin_name, path));
        }
    }

    Ok(registered_count)
}

fn is_plugin_file<P: AsRef<Path>>(path: P) -> Option<String> {
    let path = path.as_ref();
    if !path.is_file() {
        return None;
    }

    let filename = path.file_name()?.to_str()?;

    if filename.starts_with("lib") && filename.ends_with(".so") {
        let name = &filename[3..filename.len() - 3];
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }

    None
}

fn load_plugin<P: AsRef<Path>>(plugin_path: P) -> Option<Arc<CommandRef>> {
    let path = plugin_path.as_ref();

    // Load lib header
    let lib = abi_stable::library::lib_header_from_path(path).ok()?;

    let module = lib.init_root_module::<CommandRef>().ok()?;

    module.load()();

    debug!("Loaded plugin: {}", module.plugin_name()().clone());

    Some(Arc::new(module))
}
