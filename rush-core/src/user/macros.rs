use std::{env, path::PathBuf, sync::OnceLock};

macro_rules! define_user_dir {
    ($name:ident, $static_name:ident, $path_fragment:expr, $doc:expr) => {
        #[doc = $doc]
        static $static_name: OnceLock<anyhow::Result<String>> = OnceLock::new();

        paste::paste! {
            #[doc = "Get " $doc]
            pub fn [<get_ $name>]() -> anyhow::Result<String> {
                $static_name
                    .get_or_init(|| {
                        let home = env::var("HOME")
                            .map(PathBuf::from)
                            .map_err(|_| anyhow::anyhow!("HOME environment variable is not set"))?;
                        Ok(home.join($path_fragment).to_string_lossy().to_string())
                    })
                    .as_ref()
                    .map(|s| s.clone())
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
