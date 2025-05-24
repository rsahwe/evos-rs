#![feature(exit_status_error)]

use std::{borrow::Cow, env, error::Error, io::{BufWriter, Write}, path::{Path, PathBuf}, process::{Command, Stdio}};

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
            "sun8x16" => "pub type Font = crate::text::font::Sun8x16;",
            font => Err(format!("config::framebuffer::font: Invalid font {}", font))?
        })?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ModulesConfig {
    enable_ps2: bool,
    enable_sata: bool,
}

impl ModulesConfig {
    fn write_to_file(self, _file: &mut BufWriter<std::fs::File>) -> Result<(), Box<dyn Error>> {
        if self.enable_ps2 {
            println!("cargo::rustc-cfg=module_ps2");
        }

        if self.enable_sata {
            println!("cargo::rustc-cfg=module_sata");
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct KeyboardConfig {
    layout: String,
}

impl KeyboardConfig {
    fn write_to_file(self, file: &mut BufWriter<std::fs::File>) -> Result<(), Box<dyn Error>> {
        writeln!(file, "{}", match self.layout.as_str() {
            "en" => "pub type Layout = pc_keyboard::layouts::Us104Key;\npub const fn new_layout() -> pc_keyboard::layouts::Us104Key { pc_keyboard::layouts::Us104Key }",
            "de" => "pub type Layout = pc_keyboard::layouts::De105Key;\npub const fn new_layout() -> pc_keyboard::layouts::De105Key { pc_keyboard::layouts::De105Key }",
            layout => Err(format!("config::keyboard::layout: Invalid layout {}", layout))?
        })?;
        
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct KernelConfig {
    framebuffer: FrameBufferConfig,
    modules: ModulesConfig,
    keyboard: KeyboardConfig,
    log_level: String,
}

impl KernelConfig {
    fn write_to_file(self, file: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        let mut file = BufWriter::new(std::fs::File::create(file.as_ref())?);
        let file = &mut file;

        conf_dep!(self, file, framebuffer);
        conf_dep!(self, file, modules);
        conf_dep!(self, file, keyboard);

        writeln!(file, "#[derive(PartialOrd, Ord, PartialEq, Eq)]\npub enum LogLevel {{\n    Critical,Error,Warn,Info,Debug\n}}")?;
        writeln!(file, "pub const LOG_LEVEL: LogLevel = {};", match self.log_level.as_str() {
            "debug" => "LogLevel::Debug",
            "info" => "LogLevel::Info",
            "warn" => "LogLevel::Warn",
            "error" => "LogLevel::Error",
            "critical" => "LogLevel::Critical",
            _ => Err(format!("config::LOG_LEVEL: Invalid level {}", self.log_level))?
        })?;
        writeln!(file, "pub const KERNEL_ID: &'static str = \"{}\";", std::env::var("KERNEL_ID").unwrap_or("DEFAULT".to_string()))?;

        Ok(())
    }
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=config/default.toml");
    println!("cargo::rerun-if-changed=config/local.toml");

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

    let mut git_branch = Command::new("git");
    let git_branch = git_branch.args(["rev-parse", "--abbrev-ref", "HEAD"]).stdout(Stdio::piped());

    let git_branch = git_branch.output().unwrap();
    let git_branch = git_branch.exit_ok();
    let git_branch = match git_branch {
        Ok(ref out) => String::from_utf8_lossy(&out.stdout),
        Err(_) => Cow::Borrowed("detached"),
    };

    println!("cargo::rustc-env=EVOS_BUILD_ID={}", git_branch);
    println!("cargo::rustc-env=EVOS_BUILD_PROFILE={}", std::env::var("PROFILE").unwrap());
}
