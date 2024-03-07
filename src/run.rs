use crate::{build, package::YacToml, update::src_files_updated};
use std::{error::Error, path::Path, process::Command};



pub async fn run() -> Result<(), Box<dyn Error>> {
    const TARGET: &str = "target";

    build::build().await?;

    let yac_toml = toml::from_str::<YacToml>(
        &tokio::fs::read_to_string("Yac.toml").await?,
    )?;

    let executable_name = yac_toml.package.name;
    let mut executable_path = Path::new(TARGET)
        .join("debug")
        .join(executable_name);

    executable_path.set_extension("exe");

    Command::new(executable_path).spawn()?.wait()?;

    Ok(())
}