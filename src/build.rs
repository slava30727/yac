use crate::dynamic::{Build, Builder, BuilderCreationError, CBuild};
use crate::package::*;
use crate::update::*;
use std::{path::{Path, PathBuf}, fs, process::{Command, ExitStatus}, error::Error};



pub async fn build(release: bool) -> Result<ExitStatus, Box<dyn Error>> {
    use path_absolutize::Absolutize;
    use crate::prettify::print_aligned;
    use std::time::Instant;

    const TARGET: &str = "target";

    if !Path::new(TARGET).exists() {
        fs::create_dir(TARGET)?;
    }

    let yac_toml = toml::from_str::<YacToml>(
        &tokio::fs::read_to_string("Yac.toml").await?
    )?;

    let src_path = std::env::current_dir()?;

    print_aligned(
        "Compiling",
        &format!(
            "{name} v{version} ({path})",
            name = yac_toml.package.name,
            version = yac_toml.package.version,
            path = src_path.absolutize()?.to_str().unwrap(),
        ),
    )?;

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
        Some(build) => BuildCfg::from_dynamic(build, &yac_toml.package.name, release),
        None => BuildCfg {
            target: PathBuf::from(TARGET),
            src_files: vec![],
            executable_name: PathBuf::from(&yac_toml.package.name),
            package_name: yac_toml.package.name.clone(),
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
pub struct BuildCfg {
    target: PathBuf,
    src_files: Vec<PathBuf>,
    executable_name: PathBuf,
    package_name: String,
    release: bool,
}

impl BuildCfg {
    pub fn from_dynamic(
        build: Build, package_name: impl Into<String>, release: bool,
    ) -> Self {
        let target = PathBuf::from("target");
        let src_files = build.src_files.into_iter().map(PathBuf::from).collect();
        let out_file = PathBuf::from(build.executable_name);

        Self {
            target,
            src_files,
            executable_name: out_file,
            release,
            package_name: package_name.into(),
        }
    }
}

async fn run_build(cfg: BuildCfg) -> Result<ExitStatus, Box<dyn Error>> {
    if !src_files_updated(cfg.release).await {
        return Ok(ExitStatus::default());
    }

    let app_path = cfg.target.join(if cfg.release { "release" } else { "debug" });

    if !app_path.exists() {
        fs::create_dir_all(&app_path)?;
    }

    let status = Command::new("gcc")
        .args(["-Wall", "-Wextra", "-Wpedantic"])
        .arg("src/*.c")
        .arg(if cfg.release { "-O3" } else { "-g" })
        .arg("-o")
        .arg(app_path.join(cfg.executable_name).with_extension("exe"))
        .spawn()?
        .wait()?;

    Ok(status)
}