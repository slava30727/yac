use path_absolutize::Absolutize;

use crate::dynamic::{Build, Builder, BuilderCreationError, CBuild};
use crate::lsp::{Clangd, YAC_INCLUDE_PATH};
use crate::prettify::print_aligned;
use crate::yac_toml::*;
use crate::update::*;
use std::io::Write;
use std::{path::{Path, PathBuf}, fs, process::{Command, ExitStatus}, error::Error};
use rayon::prelude::*;



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

const WARNING_FLAGS: &[&str] = &[
    "-Wall", "-Wextra", "-Wdouble-promotion", "-Wformat-overflow=2",
    "-Wformat-nonliteral", "-Wformat-security",
];

async fn run_build(cfg: BuildCfg<'_>) -> Result<ExitStatus, Box<dyn Error>> {
    let yac_toml_updated = !yac_toml_updated().await;

    let yac_lock = if !Path::new("Yac.lock").exists() || yac_toml_updated {
        let yac_lock = YacLock::linearize_from(cfg.yac_toml, "./")?;
        yac_lock.write("./").await?;
        yac_lock
    } else {
        YacLock::read("./").await?
            .expect("Yac.lock should be in current directory")
    };

    let include_flags = cfg.yac_toml.dependencies.values().map(|dependancy| {
        let Location::Path { ref path } = dependancy.location else {
            todo!("Location::Link is not supported yet");
        };

        let include = Path::new(path).join("include");
        let include = include.absolutize().unwrap();

        format!("-I{}", include.display())
    }).collect::<Vec<_>>();

    if yac_toml_updated || build_file_updated().await {
        let mut clangd = Clangd::read("./").await.unwrap_or_default();

        clangd.compile_flags.add.values.clear();

        if Path::new("build.c").exists() {
            clangd.add_include_path(YAC_INCLUDE_PATH);
        }

        clangd.compile_flags.add.values.extend_from_slice(&include_flags);

        clangd.write("./").await?;
    }

    if !src_files_updated(cfg.release).await {
        return Ok(ExitStatus::default());
    }

    let app_path = cfg.target.join(
        if cfg.release { "release" } else { "debug" }
    );

    if !app_path.exists() {
        fs::create_dir_all(&app_path)?;
    }

    let artifacts_dir = app_path.join(&cfg.yac_toml.package.name);

    let dependancy_out
        = build_dependencies(&yac_lock.package, &artifacts_dir, &cfg);

    std::io::stderr().write_all(&dependancy_out.stderr)?;

    if let Some(&code) = dependancy_out.exit_codes.iter().find(|code| !code.success()) {
        return Ok(code);
    }

    let libs = yac_lock.package.iter()
        .map(|target| {
            artifacts_dir
                .join(&target.yac_toml.package.name)
                .join("*.o")
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
        .args(&include_flags)
        .args(libs)
        .arg(if cfg.release { "-O3" } else { "-g" })
        .arg("-o")
        .arg(app_path.join(cfg.executable_name).with_extension("exe"))
        .spawn()?
        .wait()?;

    Ok(status)
}



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DependacyBuildOutput {
    pub stderr: Vec<u8>,
    pub exit_codes: Vec<ExitStatus>,
}

impl std::fmt::Display for DependacyBuildOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(std::str::from_utf8(&self.stderr).unwrap())
    }
}



/// TODO: run dependancies build scripts
pub fn build_dependencies<'t>(
    targets: impl IntoIterator<Item = &'t Target>,
    artifacts_dir: impl AsRef<Path>,
    cfg: &BuildCfg<'_>,
) -> DependacyBuildOutput {
    let artifacts_dir = artifacts_dir.as_ref();

    let commands = targets.into_iter().flat_map(|target| {
        let Location::Path { ref path } = target.description.location else {
            todo!("Location::Link is not supported yet");
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

        let mut commands = vec![];

        for dir in walkdir::WalkDir::new(Path::new(path).join("src")).into_iter().flatten() {
            use std::ffi::OsStr;

            let Some(extension) = dir.path().extension() else { continue };

            if extension != OsStr::new("c") {
                continue;
            }

            let out_name = dir.path().to_str().unwrap().replace(['/', '\\', '.'], "_");

            let mut command = Command::new("gcc");

            command
                .args(WARNING_FLAGS)
                .arg("-c")
                .arg(dir.path())
                .arg(if cfg.release { "-O3" } else { "-g" })
                .arg("-o")
                .arg(cur_dir.join(&out_name).with_extension("o"))
                .stdout(std::process::Stdio::piped());

            commands.push((cur_dir.clone(), command));
        }

        Some(commands)
    }).flatten().collect::<Vec<_>>();
    
    let mut handles = Vec::with_capacity(commands.len());

    commands.into_par_iter()
        .map(|(path, mut command)| (path, command.output().unwrap()))
        .collect_into_vec(&mut handles);

    let mut exit_codes = Vec::with_capacity(handles.len());
    let mut stderr = Vec::with_capacity(handles.len());

    for (path, mut output) in handles {
        if path.exists() && !output.status.success() {
            std::fs::remove_dir_all(&path).unwrap();
        }

        exit_codes.push(output.status);
        stderr.append(&mut output.stderr);
    }

    DependacyBuildOutput { stderr, exit_codes }
}