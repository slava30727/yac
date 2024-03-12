use serde::{Serialize, Deserialize};
use std::path::Path;



pub const YAC_INCLUDE_PATH: &str = r"D:\Svyatoslav\Programs\yac\include";



#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Clangd {
    pub compile_flags: CompileFlags,
}

impl Clangd {
    pub async fn read(location: impl AsRef<Path>) -> Result<Self, ClangdError> {
        let from = location.as_ref().join(".clangd");

        let result = serde_yaml::from_str(
            &tokio::fs::read_to_string(&from).await?
        )?;

        Ok(result)
    }

    pub async fn write(&self, location: impl AsRef<Path>) -> Result<(), ClangdError> {
        tokio::fs::write(
            location.as_ref().join(".clangd"),
            &serde_yaml::to_string(self)?
        ).await?;

        Ok(())
    }

    pub fn add_include_path(&mut self, path: impl AsRef<Path>) {
        self.compile_flags.add.values.push(
            format!("-I{}", path.as_ref().display()),
        );
    }
}

impl Default for Clangd {
    fn default() -> Self {
        Self {
            compile_flags: CompileFlags {
                add: Add {
                    values: vec![format!("-I{}", YAC_INCLUDE_PATH)],
                },
            },
        }
    }
}



#[derive(Debug, thiserror::Error)]
pub enum ClangdError {
    #[error(transparent)]
    IoError(#[from] tokio::io::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_yaml::Error),
}



#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Add<T> {
    pub values: Vec<T>,
}



#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CompileFlags {
    pub add: Add<String>,
}



#[cfg(test)]
mod tests {
    use super::*;

    const CLANGD: &str = r#"CompileFlags:
  Add:
  - -ID:\Svyatoslav\Programs\yac\include
"#;

    #[test]
    fn de() {
        let clangd = serde_yaml::from_str::<Clangd>(CLANGD).unwrap();

        assert_eq!(clangd, Clangd::default());
    }

    #[test]
    fn ser() {
        let clangd = serde_yaml::to_string(&Clangd::default()).unwrap();

        eprintln!("{clangd}");

        assert_eq!(clangd, CLANGD);
    }
}