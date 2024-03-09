use std::{error::Error, process::Command};
use crate::{package::YacToml, prettify::print_aligned};



pub async fn clean() -> Result<(), Box<dyn Error>> {
    let yac_toml = toml::from_str::<YacToml>(
        &tokio::fs::read_to_string("Yac.toml").await?
    )?;

    print_aligned("Cleaning", &format!("package {} artifacts", yac_toml.package.name))?;

    Command::new("rm")
        .args(["-rf", "target"])
        .spawn()?
        .wait()?;

    Ok(())
}