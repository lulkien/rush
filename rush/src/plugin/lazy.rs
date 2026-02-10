use std::{
    fs::{self, File},
    io::Read,
    path::Path,
    sync::Arc,
};

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

// pub fn reload_plugin(name: &str) -> anyhow::Result<Arc<CommandRef>> {
//     let mut registry_writer = write_plugin_registry()?;
//
//     let metadata_mut = registry_writer
//         .borrow_mut(name)
//         .ok_or_else(|| anyhow::anyhow!("{}: command not found", name))?;
//
//     metadata_mut.plugin = None;
//
//     let plugin = load_plugin(&metadata_mut.path);
//     metadata_mut.plugin = plugin.clone();
//
//     plugin.ok_or_else(|| anyhow::anyhow!("{}: plugin failed to load", name))
// }

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
    let dir_path = path.as_ref();
    let mut registered_count = 0;

    ensure!(
        dir_path.exists(),
        format!("{} not found", dir_path.display())
    );
    ensure!(
        dir_path.is_dir(),
        format!("{} is not a directory", dir_path.display())
    );

    debug!("Load plugin from: {}", dir_path.display());

    let entries = fs::read_dir(dir_path)
        .with_context(|| format!("Failed to read directory: {}", dir_path.display()))?;

    for entry in entries {
        let entry =
            entry.with_context(|| format!("Failed to read entry in {}", dir_path.display()))?;
        let entry_path = entry.path();

        if is_metadata_file(&entry_path) {
            let mut file = File::open(entry_path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;

            if let Ok(metadata) = PluginMetadata::from_raw_metadata(dir_path, &buf) {
                debug!("Registered plugin path: {}", metadata.name);
                registered_count += 1;
                write_plugin_registry()?.add(&metadata.name.clone(), metadata);
            }
        }
    }

    Ok(registered_count)
}

fn is_metadata_file<P: AsRef<Path>>(path: P) -> bool {
    if let Some(extension) = path.as_ref().extension()
        && extension == "metadata"
    {
        return true;
    }
    false
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
