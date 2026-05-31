use std::{
    cell::RefCell,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use anyhow::{Context, ensure};
use log::{debug, info};
use rush_interface::CommandRef;

use crate::{
    env::EnvRegistry,
    plugin::{PluginMetadata, PluginRegistry},
    types::DashRegistry,
};

pub(super) fn load_plugin<P: AsRef<Path>>(plugin_path: P) -> Option<Rc<CommandRef>> {
    let path = plugin_path.as_ref();

    // Load lib header
    let lib = abi_stable::library::lib_header_from_path(path).ok()?;

    let module = lib.init_root_module::<CommandRef>().ok()?;

    module.load()();

    debug!("Loaded plugin: {}", module.plugin_name()().clone());

    Some(Rc::new(module))
}

pub(super) fn discover(plugin: &mut PluginRegistry, env: &EnvRegistry) -> anyhow::Result<()> {
    let mut registered_count = 0;

    env.get_variable("RUSH_DATA_PATH")?.iter().for_each(|path| {
        registered_count +=
            discover_from_path(plugin, PathBuf::from_str(path).unwrap().join("plugins"))
                .unwrap_or_default();
    });

    env.get_variable("RUSH_PLUGIN_PATH")?
        .iter()
        .for_each(|path| {
            registered_count +=
                discover_from_path(plugin, PathBuf::from_str(path).unwrap()).unwrap_or_default();
        });

    info!("Registered {} plugin(s)", registered_count);

    Ok(())
}

fn discover_from_path<P: AsRef<Path>>(
    plugin: &mut PluginRegistry,
    path: P,
) -> anyhow::Result<usize> {
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
                if !plugin.contains(&metadata.name) {
                    registered_count += 1;
                } else {
                    debug!("Overriding plugin: {}", metadata.name);
                }
                plugin.register(&metadata.name.clone(), Rc::new(RefCell::new(metadata)));
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
