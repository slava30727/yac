use std::error::Error;
use crate::package::YacToml;



const MAIN_FILE_DEFAULT: &str = r#"#include <stdio.h>
#include <stdlib.h>

int main(void) {
    printf("Hello, World!\n");

    return EXIT_SUCCESS;
}
"#;

const BUILD_FILE_DEFAULT: &str = r#"#include <yac/lib.h>

void build(Build* build) {
    Build_add_source_files(build, "src/*.c");
}
"#;

const GITIGNORE_FILE_DEFAULT: &str = "/target\n";

const CLANGD_FILE_DEFAULT: &str = r#"CompileFlags:
    Add:
        - "-ID:\\svyatoslav\\programs\\yac\\include"
"#;



pub async fn new(name: String) -> Result<(), Box<dyn Error>> {
    use std::{fs, path::PathBuf, process::Command};

    let prj_dir = PathBuf::from(&name);
    fs::create_dir_all(prj_dir.join("src"))?;

    let git_init = Command::new("git")
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

    Ok(())
}