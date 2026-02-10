use std::{
    env,
    fs::File,
    io::{self, Write},
    path::Path,
};

struct PluginMetadata {
    plugin_name: String,
    plugin_so: String,
}

impl PluginMetadata {
    fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let name_bytes = self.plugin_name.as_bytes();
        let name_len: u16 = name_bytes.len() as u16;

        let so_bytes = self.plugin_so.as_bytes();
        let so_len: u16 = so_bytes.len() as u16;

        let total_length = 2 + 2 + name_len + 2 + so_len;

        let mut buffer = Vec::new();

        buffer.write_all(&total_length.to_ne_bytes())?; // Use native endian
        buffer.write_all(&name_len.to_ne_bytes())?; // Use native endian
        buffer.write_all(name_bytes)?;
        buffer.write_all(&so_len.to_ne_bytes())?; // Use native endian
        buffer.write_all(so_bytes)?;

        Ok(buffer)
    }
}

fn main() -> io::Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap(); // target/release

    let plugin_name = env!("CARGO_PKG_NAME");
    let metadata = PluginMetadata {
        plugin_name: plugin_name.to_string(),
        plugin_so: format!("lib{}.so", plugin_name.replace("-", "_")),
    };

    let metadata_path = out_dir.join(format!("{}.metadata", env!("CARGO_PKG_NAME")));

    let mut file = File::create(metadata_path)?;

    file.write_all(&metadata.to_bytes()?)?;

    Ok(())
}
