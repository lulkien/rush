macro_rules! create_dir_registry {
    ($name:ident, $static_name:ident, $doc:expr) => {
        #[doc = $doc]
        static $static_name: OnceLock<RwLock<Vec<PathBuf>>> = OnceLock::new();

        fn $name() -> &'static RwLock<Vec<PathBuf>> {
            $static_name.get_or_init(|| RwLock::new(Vec::new()))
        }

        paste::paste! {
            #[doc = "Read " $doc]
            pub fn [<read_ $name>]() -> anyhow::Result<RwLockReadGuard<'static, Vec<PathBuf>>> {
                $name()
                    .read()
                    .map_err(|_| anyhow::anyhow!(concat!(stringify!($static_name), " read lock poisoned")))
            }

            #[doc = "Write " $doc]
            fn [<write_ $name>]() -> anyhow::Result<RwLockWriteGuard<'static, Vec<PathBuf>>> {
                $name()
                    .write()
                    .map_err(|_| anyhow::anyhow!(concat!(stringify!($static_name), " read lock poisoned")))
            }

            #[doc = "Add a directory to " $doc]
            pub fn [<add_ $name>](path: &PathBuf, prepend: bool) -> anyhow::Result<()> {
                let mut dirs = $name()
                    .write()
                    .map_err(|_| anyhow::anyhow!(concat!(stringify!($static_name), " write lock poisoned")))?;

                if !dirs.contains(&path) {
                    match prepend {
                        true => dirs.insert(0, path.to_owned()),
                        false => dirs.push(path.to_owned()),
                    }
                }

                Ok(())
            }
        }
    };
}

pub(super) use create_dir_registry;
