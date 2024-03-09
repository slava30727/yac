use crate::{build, package::YacToml, prettify::print_aligned};
use std::{error::Error, path::Path, process::Command};



pub async fn run(release: bool) -> Result<(), Box<dyn Error>> {
    const TARGET: &str = "target";

    let build_status = build::build(release).await?;

    if !build_status.success() {
        return Ok(());
    }

    let yac_toml = toml::from_str::<YacToml>(
        &tokio::fs::read_to_string("Yac.toml").await?,
    )?;

    let executable_name = yac_toml.package.name;
    let mut executable_path = Path::new(TARGET)
        .join(if release { "release" } else { "debug" })
        .join(executable_name);

    executable_path.set_extension("exe");

    print_aligned(
        "Running",
        &format!("`{}`", executable_path.to_str().unwrap()),
    )?;

    Command::new(executable_path).spawn()?.wait()?;

    Ok(())
}