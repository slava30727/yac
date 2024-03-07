use std::{error::Error, process::Command};



pub fn clean() -> Result<(), Box<dyn Error>> {
    Command::new("rm")
        .arg("-rf")
        .spawn()?
        .wait()?;

    Ok(())
}