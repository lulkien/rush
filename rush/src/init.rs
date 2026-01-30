#![allow(unused)]

use log::{info, warn};
use std::{env, fs, path::PathBuf, sync::OnceLock};

static USER_CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();
static USER_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static USER_CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn get_user_config_dir() -> &'static PathBuf {
    USER_CONFIG_DIR.get_or_init(|| {
        let config_dir = get_xdg_dir("XDG_CONFIG_HOME", ".config")
            .expect("Cannot get user config dir")
            .join("rush");

        info!("User config directory: {}", config_dir.display());
        config_dir
    })
}

pub fn get_user_data_dir() -> &'static PathBuf {
    USER_DATA_DIR.get_or_init(|| {
        let data_dir = get_xdg_dir("XDG_DATA_HOME", ".local/share")
            .expect("Cannot get user data dir")
            .join("rush");

        info!("User data directory: {}", data_dir.display());
        data_dir
    })
}

pub fn get_user_cache_dir() -> &'static PathBuf {
    USER_CACHE_DIR.get_or_init(|| {
        let cache_dir = get_xdg_dir("XDG_CACHE_HOME", ".cache")
            .expect("Cannot get user cache dir")
            .join("rush");

        info!("User cache directory: {}", cache_dir.display());
        cache_dir
    })
}

fn get_xdg_dir(env_var: &str, fallback_relative: &str) -> anyhow::Result<PathBuf> {
    if let Ok(dir) = env::var(env_var) {
        let path = PathBuf::from(dir);
        if path.is_absolute() {
            return Ok(path);
        }
        warn!("{} is not an absolute path: {}", env_var, path.display());
    }

    let home_dir = get_home_dir()?;
    Ok(home_dir.join(fallback_relative))
}

fn get_home_dir() -> anyhow::Result<PathBuf> {
    env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("HOME variable is not set"))
}

pub fn init_shell() -> anyhow::Result<()> {
    // Init logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();

    // Create user config dir on first run
    create_user_config_dir()?;

    Ok(())
}

fn create_user_config_dir() -> anyhow::Result<()> {
    let user_config_dir = get_user_config_dir();

    if !user_config_dir.exists() {
        fs::create_dir(user_config_dir)?;
        info!("Create {}", user_config_dir.display());
    }

    Ok(())
}
