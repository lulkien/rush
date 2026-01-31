use std::{
    collections::HashMap,
    fs,
    path::Path,
    sync::{OnceLock, RwLock},
};

use anyhow::Context;
use log::{error, info};
use rush_interface::CommandRef;

use crate::{commands, env::read_rush_data_dirs, executor::ExecutorWrapper};

// -------------------------------- Plugin registry --------------------------------

#[derive(Default)]
struct PluginRegistry(HashMap<String, CommandRef>);

static PLUGIN_REGISTRY: OnceLock<RwLock<PluginRegistry>> = OnceLock::new();

fn get_plugin_registry() -> &'static RwLock<PluginRegistry> {
    PLUGIN_REGISTRY.get_or_init(|| RwLock::new(PluginRegistry::default()))
}

fn register_plugin(name: String, command_ref: CommandRef) -> anyhow::Result<()> {
    let registry = get_plugin_registry();

    let mut registry = registry
        .write()
        .map_err(|_| anyhow::anyhow!("Plugin registry lock poisoned"))?;

    registry.0.insert(name, command_ref);
    Ok(())
}

pub(crate) fn get_plugin(plugin_name: &str) -> anyhow::Result<CommandRef> {
    let registry = get_plugin_registry();

    let registry = registry
        .read()
        .map_err(|e| anyhow::anyhow!("Failed to acquire read lock: {e}"))?;

    registry
        .0
        .get(plugin_name)
        .cloned()
        .with_context(|| format!("Plugin '{}' not found", plugin_name))
}

// -------------------------------- Plugin subsystem --------------------------------

pub(crate) fn init_module() -> anyhow::Result<()> {
    let mut plugins_loaded = 0;

    read_rush_data_dirs()?.iter().for_each(|path| {
        info!("{}", &path.join("plugins").display());
        plugins_loaded += load_plugins_from_directory(&path.join("plugins")).unwrap_or_default()
    });

    info!("Loaded {} plugins", plugins_loaded);

    update_shell_commands()?;

    Ok(())
}

fn load_plugins_from_directory(path: &Path) -> anyhow::Result<usize> {
    anyhow::ensure!(path.exists(), format!("{} not found", path.display()));
    anyhow::ensure!(
        path.is_dir(),
        format!("{} is not a directory", path.display())
    );

    let mut loaded = 0;

    let entries = fs::read_dir(path)
        .with_context(|| format!("Failed to read directory: {}", path.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("Failed to read entry in {}", path.display()))?;
        let path = entry.path();

        if is_plugin_file(&path) {
            match load_plugin(&path) {
                Ok(_) => {
                    info!("Loaded plugin: {}", path.display());
                    loaded += 1;
                }
                Err(e) => {
                    error!("{}", e);
                }
            }
        }
    }

    Ok(loaded)
}

fn is_plugin_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    let extension = path.extension().and_then(|ext| ext.to_str());

    match extension {
        Some("so") if cfg!(unix) => true,
        // Some("dylib") if cfg!(target_os = "macos") => true,
        // Some("dll") if cfg!(windows) => true,
        _ => false,
    }
}

fn load_plugin(path: &Path) -> anyhow::Result<()> {
    let lib = abi_stable::library::lib_header_from_path(path)
        .with_context(|| format!("Failed to load library: {}", path.display()))?;

    let module = lib
        .init_root_module::<CommandRef>()
        .with_context(|| format!("Failed to get root module from: {}", path.display()))?;

    module.load()();
    let plugin_name = module.info()().name;

    register_plugin(plugin_name.into(), module)?;
    Ok(())
}

fn update_shell_commands() -> anyhow::Result<()> {
    if let Ok(plugin) = get_plugin("rush_prompt") {
        let mut shell_commands = commands::lock_shell_commands_write()?;

        shell_commands.set_prompt(ExecutorWrapper::new(plugin.exec()));
    }

    Ok(())
}
