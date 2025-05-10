use std::{env, error::Error, io::{BufWriter, Write}, path::{Path, PathBuf}};

use config::{Config, File};
use serde::Deserialize;

macro_rules! conf_dep {
    ($self:ident, $file:ident, $name:ident) => {
        writeln!($file, "pub mod {} {{", stringify!($name))?;
        $self.$name.write_to_file($file)?;
        writeln!($file, "}}")?;
    };
}

#[derive(Debug, Deserialize)]
struct FrameBufferConfig {
    font: String,
}

impl FrameBufferConfig {
    fn write_to_file(self, file: &mut BufWriter<std::fs::File>) -> Result<(), Box<dyn Error>> {
        writeln!(file, "{}", match self.font.as_str() {
            "basic8x8" => "pub type Font = crate::text::font::Basic8x8;",
            "ter16x32" => "pub type Font = crate::text::font::Ter16x32;",
            font => Err(format!("config::framebuffer::font: Invalid font {}", font))?
        })?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct KernelConfig {
    framebuffer: FrameBufferConfig,
}

impl KernelConfig {
    fn write_to_file(self, file: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        let mut file = BufWriter::new(std::fs::File::create(file.as_ref())?);
        let file = &mut file;

        conf_dep!(self, file, framebuffer);

        Ok(())
    }
}

fn main() {
    let kc = Config::builder()
        .add_source(File::with_name("config/default"))
        .add_source(File::with_name("config/local").required(false))
        .build()
        .expect("Could not load config!");

    let kc = kc
        .try_deserialize::<KernelConfig>()
        .expect("Could not deserialize config!");

    let out_path = PathBuf::from(env::var("OUT_DIR")
        .unwrap());

    kc
        .write_to_file(out_path.join("config.rs"))
        .expect("Could not write config!");
}
