use std::error::Error;
use crate::{yac_toml::YacToml, prettify::{print_aligned, error}};



pub async fn clean() -> Result<(), Box<dyn Error>> {
    let Some(yac_toml) = YacToml::read("./").await? else {
        error(
            "failed to locate project in current directory", None,
        )?;

        return Ok(());
    };

    print_aligned("Cleaning", &format!("package `{}`'s artifacts", yac_toml.package.name))?;

    tokio::try_join!(
        tokio::fs::remove_dir_all("target"),
        tokio::fs::remove_file("Yac.lock"),
    )?;

    Ok(())
}