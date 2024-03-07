use crate::dynamic::{Build, Builder, CBuild};
use crate::package::*;
use crate::update::*;
use std::ffi::OsString;
use std::{path::{Path, PathBuf}, fs, process::Command, error::Error};



pub async fn build() -> Result<(), Box<dyn Error>> {
    const TARGET: &str = "target";

    if !Path::new(TARGET).exists() {
        fs::create_dir(TARGET);
    }

    build_impl(get_build().await?.into()).await;

    Ok(())
}

pub async fn get_build() -> Result<Build, Box<dyn Error>> {
    const YAC_TOML: &str = "Yac.toml";
    const BUILD: &str = "build.c";

    let yac_toml = toml::from_str::<YacToml>(
        &tokio::fs::read_to_string(YAC_TOML).await?,
    )?;

    let mut build_values = if Path::new(BUILD).exists() {
        if build_file_updated().await {
            Builder::compile("build.c", "target/build.dll");
        }

        let api = Builder::new("target/build.dll")?;
        let mut cbuild = CBuild::new(api);
        cbuild.build();
        
        Build::from(&cbuild)
    } else {
        Build {
            src_files: vec![OsString::from("src/*.c")],
            executable_name: yac_toml.package.name.as_str().into(),
            link_directories: vec![],
            enabled_flags: vec![OsString::from("DEBUG")],
        }
    };

    if build_values.executable_name.is_empty() {
        build_values.executable_name = yac_toml.package.name.into();
    }

    if !build_values.src_files.contains(&"src/*.c".into()) {
        build_values.src_files.push(OsString::from("src/*.c"));
    }

    Ok(build_values)
}

#[derive(Clone, Debug)]
pub struct BuildCfg {
    target: PathBuf,
    src_files: Vec<PathBuf>,
    out_file: PathBuf,
    release: bool,
}

impl From<Build> for BuildCfg {
    fn from(value: Build) -> Self {
        let target = PathBuf::from("target");
        let src_files = value.src_files.into_iter().map(PathBuf::from).collect();
        let out_file = PathBuf::from(value.executable_name);
        let release = false;

        Self { target, src_files, out_file, release }
    }
}

async fn build_impl(cfg: BuildCfg) {
    let target = Path::new("target");

    if !target.exists() {
        fs::create_dir("target").unwrap();
    }

    if !src_files_updated().await {
        return;
    }

    let exe_path = if cfg.release {
        let path = target.join("release");

        if !path.exists() {
            fs::create_dir(&path).unwrap();
        }

        path
    } else {
        let path = target.join("debug");

        if !path.exists() {
            fs::create_dir(&path).unwrap();
        }

        path
    };

    println!("\t[INFO : Building project]");

    Command::new("gcc")
        .args(cfg.src_files)
        .args(["-g", "-o"])
        .arg(exe_path.join(cfg.out_file))
        .spawn()
        .unwrap()
        .wait()
        .unwrap();
}