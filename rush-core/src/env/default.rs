use crate::{env::EnvRegistry, user::UserDirectoryRegistry};

pub(super) fn setup_path(
    env: &mut EnvRegistry,
    user_dirs: &UserDirectoryRegistry,
) -> anyhow::Result<()> {
    const SYSTEM_LOCAL_DATA_PATH: &str = "/usr/local/share/rush";
    const SYSTEM_DATA_PATH: &str = "/usr/share/rush";
    const SYSTEM_CONFIG_PATH: &str = "/etc/rush";

    let data_path = vec![
        user_dirs.get_data_dir(),
        SYSTEM_LOCAL_DATA_PATH.to_string(),
        SYSTEM_DATA_PATH.to_string(),
    ];
    env.set_variable("RUSH_DATA_PATH", data_path);

    let config_path = vec![user_dirs.get_config_dir(), SYSTEM_CONFIG_PATH.to_string()];
    env.set_variable("RUSH_CONFIG_PATH", config_path);

    let cache_path = vec![user_dirs.get_cache_dir()];
    env.set_variable("RUSH_CACHE_PATH", cache_path);

    Ok(())
}
