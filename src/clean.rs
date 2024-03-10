use std::{error::Error, process::Command};
use crate::{yac_toml::YacToml, prettify::{print_aligned, error}};



pub async fn clean() -> Result<(), Box<dyn Error>> {
    let Some(yac_toml) = YacToml::read("./").await? else {
        error(
            "failed to locate project in current directory", None,
        )?;

        return Ok(());
    };

    print_aligned("Cleaning", &format!("package `{}`'s artifacts", yac_toml.package.name))?;

    Command::new("rm")
        .args(["-rf", "target"])
        .spawn()?
        .wait()?;

    Ok(())
}