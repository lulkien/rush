use std::{
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::ensure;
use rush_interface::CommandRef;

#[derive(Clone)]
pub struct PluginMetadata {
    pub(super) name: String,
    pub(super) path: PathBuf,
    pub(super) plugin: Option<Rc<CommandRef>>,
}

impl PluginMetadata {
    pub fn is_loaded(&self) -> bool {
        self.plugin.is_some()
    }

    pub fn from_raw_metadata<P: AsRef<Path>>(metadata_path: P, buf: &[u8]) -> anyhow::Result<Self> {
        let mut pos: usize = 0;

        let total_len = buf.len();

        // Extract plugin name length from buffer[0..2]
        let header = u16::from_ne_bytes(buf[pos..pos + 2].try_into()?) as usize;
        pos += 2;

        // Ensure buffer has valid length
        ensure!(header == total_len, "Invalid buffer length");

        // Extract plugin name header from buffer[2..4]
        let name_len = u16::from_ne_bytes(buf[pos..pos + 2].try_into()?) as usize;
        pos += 2;

        // Ensure plugin name header is valid
        ensure!(total_len > pos + name_len, "Invalid plugin name header");

        // Extract plugin name
        let name = String::from_utf8(buf[pos..pos + name_len].to_vec())?;
        pos += name_len;

        // Extract plugin filename header from next 2 bytes
        let filename_len = u16::from_ne_bytes(buf[pos..pos + 2].try_into()?) as usize;
        pos += 2;

        // Ensure plugin filename header is valid
        ensure!(
            total_len == pos + filename_len,
            "Invalid plugin name header"
        );

        let filename = String::from_utf8(buf[pos..pos + filename_len].to_vec())?;
        // pos += filename_len; // never read again

        Ok(Self {
            name,
            path: metadata_path.as_ref().join(filename),
            plugin: None,
        })
    }
}
