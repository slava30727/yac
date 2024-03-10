use std::error::Error;
use crate::{yac_toml::YacToml, prettify::{print_aligned, error}};



const MAIN_FILE_DEFAULT: &str = r#"#include <stdio.h>
#include <stdlib.h>

int main(void) {
    printf("Hello, World!\n");

    return EXIT_SUCCESS;
}
"#;

const GITIGNORE_FILE_DEFAULT: &str = "/target\n";

const CLANGD_FILE_DEFAULT: &str = r#"CompileFlags:
    Add:
        - "-ID:\\svyatoslav\\programs\\yac\\include"
"#;



pub async fn new(name: &str) -> Result<(), Box<dyn Error>> {
    use std::{fs, path::PathBuf, process::Command};
    use path_absolutize::Absolutize;

    let prj_dir = PathBuf::from(name);

    if prj_dir.exists() {
        // TODO: fix canonicalize strange behavior
        // TODO: add `yac init`
        error(
            &format!("destination `{}` already exists", prj_dir.absolutize()?.to_str().unwrap()),
            Some("Use `yac init` to initialize the directory"),
        )?;

        return Ok(());
    }

    fs::create_dir_all(prj_dir.join("src"))?;

    Command::new("git")
        .current_dir(&prj_dir)
        .arg("init")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    let yac_toml = YacToml::new(name);
    let yac_toml_string = toml::to_string(&yac_toml)?;

    tokio::try_join! {
        tokio::fs::write(prj_dir.join("Yac.toml"), &yac_toml_string),
        tokio::fs::write(prj_dir.join(".gitignore"), GITIGNORE_FILE_DEFAULT),
        tokio::fs::write(prj_dir.join("src/main.c"), MAIN_FILE_DEFAULT),
        tokio::fs::write(prj_dir.join(".clangd"), CLANGD_FILE_DEFAULT),
    }?;

    print_aligned(
        "Created",
        &format!("binary (application) `{name}` package"),
    )?;

    Ok(())
}