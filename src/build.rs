use path_absolutize::Absolutize;

use crate::dynamic::{Build, Builder, BuilderCreationError, CBuild};
use crate::prettify::print_aligned;
use crate::yac_toml::*;
use crate::update::*;
use std::{path::{Path, PathBuf}, fs, process::{Command, ExitStatus}, error::Error};



pub fn print_compiling_message(name: &str, version: &str, path: Option<&str>) -> std::io::Result<()> {
    let loc = match path {
        Some(path) => format!(" ({})", path),
        None => String::new(),
    };

    print_aligned("Compiling", &format!("{name} v{version}{loc}"))
}

pub async fn build(release: bool) -> Result<ExitStatus, Box<dyn Error>> {
    use crate::prettify::error;
    use std::time::Instant;

    const TARGET: &str = "target";

    let Some(yac_toml) = YacToml::read("./").await? else {
        error(
            "failed to locate project in current directory",
            Some("Consider to create a new project: `yac new --name <PROJECT_NAME>`"),
        )?;

        return Ok(ExitStatus::default());
    };

    if !Path::new(TARGET).exists() {
        fs::create_dir(TARGET)?;
    }

    let time = Instant::now();

    let build_status = compile_build_script().await.unwrap_or_default();

    if !build_status.success() {
        YacUpdate::mark_build_error().await?;

        crate::prettify::error(
            &format!(
                "could not compile `{}`'s build script due to previous erorr(s)",
                yac_toml.package.name,
            ),
            None,
        )?;

        return Ok(build_status);
    }

    let build = run_build_script(&yac_toml).await?;

    let build_cfg = match build {
        Some(build) => BuildCfg::from_dynamic(build, &yac_toml, release),
        None => BuildCfg {
            target: PathBuf::from(TARGET),
            executable_name: PathBuf::from(&yac_toml.package.name),
            yac_toml: &yac_toml,
            release,
        },
    };

    let build_status = run_build(build_cfg).await?;

    let time = Instant::now().duration_since(time);

    if build_status.success() {
        print_aligned(
            "Finished",
            &format!(
                "`{mode}` profile [{profile}] target(s) in {time:.2}s",
                mode = if release { "release" } else { "debug" },
                profile = if release { "optimized" } else { "unoptimized + debuginfo" },
                time = time.as_secs_f32(),
            ),
        )?;
    } else {
        YacUpdate::mark_src_error().await?;

        crate::prettify::error(
            &format!("could not compile `{}` due to previous erorr(s)", yac_toml.package.name),
            None,
        )?;

        return Ok(build_status);
    }

    Ok(build_status)
}

pub async fn compile_build_script() -> Option<ExitStatus> {
    if !Path::new("build.c").exists() {
        return None;
    }

    Some(if build_file_updated().await {
        Builder::compile("build.c", "target/build.dll")
    } else {
        ExitStatus::default()
    })
}

pub async fn run_build_script(yac_toml: &YacToml)
    -> Result<Option<Build>, BuilderCreationError>
{
    use std::ffi::OsString;

    if !Path::new("build.c").exists() {
        return Ok(None);
    }

    let api = Builder::new("target/build.dll")?;
    let mut cbuild = CBuild::new(api);
    cbuild.build();

    let mut build = Build::from(&cbuild);

    if build.executable_name.is_empty() {
        build.executable_name = OsString::from(&yac_toml.package.name);
    }

    Ok(Some(build))
}

#[derive(Clone, Debug)]
pub struct BuildCfg<'toml> {
    target: PathBuf,
    executable_name: PathBuf,
    release: bool,
    yac_toml: &'toml YacToml,
}

impl<'toml> BuildCfg<'toml> {
    pub fn from_dynamic(
        build: Build, yac_toml: &'toml YacToml, release: bool,
    ) -> Self {
        let target = PathBuf::from("target");
        let out_file = PathBuf::from(build.executable_name);

        Self {
            target,
            executable_name: out_file,
            release,
            yac_toml,
        }
    }
}

async fn run_build(cfg: BuildCfg<'_>) -> Result<ExitStatus, Box<dyn Error>> {
    const WARNING_FLAGS: &[&str] = &[
        "-Wall", "-Wextra", "-Wdouble-promotion", "-Wformat-overflow=2",
        "-Wformat-nonliteral", "-Wformat-security",
    ];

    if !src_files_updated(cfg.release).await {
        return Ok(ExitStatus::default());
    }

    let yac_lock = if !Path::new("Yac.lock").exists()
        || !yac_toml_updated().await
    {
        let yac_lock = YacLock::linearize_from(cfg.yac_toml, "./")?;
        yac_lock.write("./").await?;
        yac_lock
    } else {
        YacLock::read("./").await?
            .expect("Yac.lock should be in current directory")
    };

    let app_path = cfg.target.join(
        if cfg.release { "release" } else { "debug" }
    );

    if !app_path.exists() {
        fs::create_dir_all(&app_path)?;
    }

    let artifacts_dir = app_path.join(&cfg.yac_toml.package.name);

    let handles = yac_lock.package.iter().flat_map(|target| {
        let Location::Path { ref path } = target.description.location else {
            todo!("Location::link is not supported yet");
        };

        let cur_dir = artifacts_dir.join(&target.yac_toml.package.name);

        if cur_dir.exists() {
            return None;
        }

        std::fs::create_dir_all(&cur_dir).unwrap();

        print_compiling_message(
            &target.yac_toml.package.name,
            &target.yac_toml.package.version,
            None,
        ).unwrap();

        Some(
            Command::new("gcc")
                .args(WARNING_FLAGS)
                .arg("-c")
                .arg(format!("{path}/src/*.c"))
                .arg(if cfg.release { "-O3" } else { "-g" })
                .arg("-o")
                .arg(cur_dir.join("out.lib"))
                .spawn()
                .unwrap()
        )
    });

    for mut handle in handles {
        handle.wait()?;
    }

    let libs = yac_lock.package.iter()
        .map(|target| {
            artifacts_dir
                .join(&target.yac_toml.package.name)
                .join("out.lib")
        });

    let src_path = std::env::current_dir()?;

    print_compiling_message(
        &cfg.yac_toml.package.name,
        &cfg.yac_toml.package.version,
        Some(src_path.absolutize()?.to_str().unwrap()),
    )?;

    let status = Command::new("gcc")
        .args(WARNING_FLAGS)
        .arg("src/*.c")
        .args(libs)
        .arg(if cfg.release { "-O3" } else { "-g" })
        .arg("-o")
        .arg(app_path.join(cfg.executable_name).with_extension("exe"))
        .spawn()?
        .wait()?;

    Ok(status)
}