#![allow(unused)]

mod macros;

use std::{
    path::PathBuf,
    sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use macros::create_dir_registry;

create_dir_registry!(rush_data_dirs, RUSH_DATA_DIRS, "Rush data directories");
create_dir_registry!(
    rush_config_dirs,
    RUSH_CONFIG_DIRS,
    "Rush configuration directories"
);

fn init_default_data_dirs() -> anyhow::Result<()> {
    let mut data_dirs = write_rush_data_dirs()?;
    let mut defaults_dirs: Vec<PathBuf> =
        vec!["/usr/local/share/rush".into(), "/usr/share/rush".into()];

    data_dirs.append(&mut defaults_dirs);

    Ok(())
}

fn init_default_config_dirs() -> anyhow::Result<()> {
    let mut config_dirs = write_rush_config_dirs()?;
    let mut defaults_dirs: Vec<PathBuf> = vec!["/etc/rush".into()];

    config_dirs.append(&mut defaults_dirs);

    Ok(())
}

pub fn init_module() -> anyhow::Result<()> {
    init_default_data_dirs()?;
    init_default_config_dirs()?;
    Ok(())
}
