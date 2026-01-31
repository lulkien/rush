#![allow(unused)]

use env_logger::{Builder, Env};
use log::{debug, info, warn};
use std::{env, fs, path::PathBuf, sync::OnceLock};

macro_rules! define_user_dir {
    ($name:ident, $static_name:ident, $path_fragment:expr, $doc:expr) => {
        #[doc = $doc]
        static $static_name: OnceLock<anyhow::Result<PathBuf>> = OnceLock::new();

        paste::paste! {
            #[doc = "Get " $doc]
            pub fn [<get_ $name>]() -> anyhow::Result<&'static PathBuf> {
                $static_name
                    .get_or_init(|| {
                        let home = env::var("HOME")
                            .map(PathBuf::from)
                            .map_err(|_| anyhow::anyhow!("HOME environment variable is not set"))?;
                        Ok(home.join($path_fragment))
                    })
                    .as_ref()
                    .map_err(|e| anyhow::anyhow!("Failed to get {}: {}", stringify!($name), e))
            }
        }
    };
}

define_user_dir!(
    user_data_dir,
    USER_DATA_DIR,
    ".local/share/rush",
    "User data directory"
);
define_user_dir!(
    user_config_dir,
    USER_CONFIG_DIR,
    ".config/rush",
    "User config directory"
);
define_user_dir!(
    user_cache_dir,
    USER_CACHE_DIR,
    ".cache/rush",
    "User cache directory"
);

pub fn init_module() -> anyhow::Result<()> {
    // Init logger
    Builder::from_env(Env::default().default_filter_or("debug")).init();

    let dirs = [
        get_user_data_dir()?,
        get_user_config_dir()?,
        get_user_cache_dir()?,
    ];

    // Create directories
    for dir in dirs {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }

    Ok(())
}
