use std::{error::Error, path::Path};
use crate::{prettify::{error, print_aligned}, yac_toml::{PackageType, YacToml}};



const MAIN_FILE_DEFAULT: &str = r#"#include <stdio.h>
#include <stdlib.h>

int main(void) {
    printf("Hello, World!\n");

    return EXIT_SUCCESS;
}
"#;

const LIBC_FILE_DEFAULT: &str = r#"int sum(int a, int b) {
    return a + b;
}

#ifdef YAC_TEST
#   include <yac/lib.h>

    void YAC_TEST_it_works(void) {
        assert(sum(2, 2) == 4 && "summation failed");
    }

#endif
"#;

const LIBH_FILE_DEFAULT: &str = r"int sum(int a, int b);";

const GITIGNORE_FILE_DEFAULT: &str = "/target\n";



pub fn create_header(name: &str, src: &str) -> String {
    let sec_macro = format!("_{}_LIB_H_", name.to_ascii_uppercase());
    format!(
        r"#ifndef {sec_macro}
#define {sec_macro}



{src}



#endif // !{sec_macro}"
    )
}

pub async fn new(name: &str, create_library: bool) -> Result<(), Box<dyn Error>> {
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

    let package_type = if create_library {
        PackageType::StaticLib
    } else {
        PackageType::Executable
    };

    let yac_toml = YacToml::new(name, package_type);
    let yac_toml_string = toml::to_string(&yac_toml)?;

    let create_default_source_file = async {
        if create_library {
            create_default_lib_files(&prj_dir, name).await
        } else {
            create_default_main_file(&prj_dir).await
        }
    };

    tokio::try_join! {
        tokio::fs::write(prj_dir.join("Yac.toml"), &yac_toml_string),
        tokio::fs::write(prj_dir.join(".gitignore"), GITIGNORE_FILE_DEFAULT),
        create_default_source_file,
    }?;

    print_aligned(
        "Created",
        &format!("{msg} `{name}` package", msg = if create_library {
            "library"
        } else {
            "binary (application)"
        }),
    )?;

    Ok(())
}

pub async fn create_default_main_file(prj_dir: impl AsRef<Path>)
    -> tokio::io::Result<()>
{
    tokio::fs::write(prj_dir.as_ref().join("src/main.c"), MAIN_FILE_DEFAULT).await
}

pub async fn create_default_lib_files(prj_dir: impl AsRef<Path>, name: &str)
    -> tokio::io::Result<()>
{
    use tokio::{try_join, io, fs};

    let prj_dir = prj_dir.as_ref();

    let include_dir = prj_dir.join("include").join(name);
    let libh_src = create_header(name, LIBH_FILE_DEFAULT);

    try_join! {
        fs::write(prj_dir.join("src/lib.c"), LIBC_FILE_DEFAULT),
        async {
            fs::create_dir_all(&include_dir).await?;
            fs::write(include_dir.join("lib.h"), &libh_src).await?;

            io::Result::<()>::Ok(())
        },
    }?;

    Ok(())
}