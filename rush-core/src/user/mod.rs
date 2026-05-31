use std::{fs, path::PathBuf, str::FromStr};

mod macros;

use macros::{get_user_cache_dir, get_user_config_dir, get_user_data_dir};

pub struct UserDirectoryRegistry {
    data_dir: String,
    config_dir: String,
    cache_dir: String,
}

#[allow(unused)]
impl UserDirectoryRegistry {
    fn new(data_dir: &str, config_dir: &str, cache_dir: &str) -> Self {
        Self {
            data_dir: data_dir.to_owned(),
            config_dir: config_dir.to_owned(),
            cache_dir: cache_dir.to_owned(),
        }
    }

    fn create_user_directories(&self) -> anyhow::Result<()> {
        let dirs = [
            PathBuf::from_str(&self.data_dir)?,
            PathBuf::from_str(&self.config_dir)?,
            PathBuf::from_str(&self.cache_dir)?,
        ];

        for dir in dirs {
            if !dir.exists() {
                fs::create_dir_all(dir)?;
            }
        }

        Ok(())
    }

    pub fn get_data_dir(&self) -> String {
        self.data_dir.clone()
    }

    pub fn get_config_dir(&self) -> String {
        self.config_dir.clone()
    }

    pub fn get_cache_dir(&self) -> String {
        self.cache_dir.clone()
    }
}

pub fn init_module() -> anyhow::Result<UserDirectoryRegistry> {
    let user_dirs = UserDirectoryRegistry::new(
        &get_user_data_dir()?,
        &get_user_config_dir()?,
        &get_user_cache_dir()?,
    );

    user_dirs.create_user_directories()?;

    Ok(user_dirs)
}
