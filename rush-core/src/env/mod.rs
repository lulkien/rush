use anyhow::anyhow;
use dashmap::{DashMap, try_result::TryResult};

use crate::user::UserDirectoryRegistry;

mod default;

#[derive(Default)]
pub struct EnvRegistry(DashMap<String, Vec<String>>);

#[allow(unused)]
impl EnvRegistry {
    pub fn get_variable(&self, name: &str) -> anyhow::Result<Vec<String>> {
        match self.0.try_get(name) {
            TryResult::Absent => Ok(vec![]),
            TryResult::Locked => Err(anyhow!("EnvRegistry is locked")),
            TryResult::Present(value) => Ok(value.clone()),
        }
    }

    pub fn set_variable(&self, name: &str, value: Vec<String>) -> Option<Vec<String>> {
        self.0.insert(name.to_owned(), value)
    }

    pub fn unset_variable(&self, name: &str) -> Option<(String, Vec<String>)> {
        self.0.remove(name)
    }
}

pub fn init_module(user_dirs: &UserDirectoryRegistry) -> anyhow::Result<EnvRegistry> {
    let mut env = EnvRegistry::default();

    default::setup_path(&mut env, user_dirs)?;

    load_exist_environment(&mut env)?;

    Ok(env)
}

fn load_exist_environment(env: &mut EnvRegistry) -> anyhow::Result<()> {
    for (key, value) in std::env::vars() {
        let parts: Vec<String> = value.split(':').map(String::from).collect();
        log::debug!("Loaded environment variable: {}: {:?}", key, parts);
        env.set_variable(&key, parts);
    }
    Ok(())
}
