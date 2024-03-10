use std::{collections::HashMap, path::Path, vec};
use serde::{Serialize, Deserialize};



#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
}



#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dependency {
    pub version: String,
    pub location: Location,
    pub r#type: Type,
}



#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Type {
    yac,
    cmake,
}



#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Location {
    path(String),
    link(String),
}



#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct YacToml {
    pub package: Package,
    pub dependencies: HashMap<String, Dependency>,
}

impl YacToml {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            package: Package {
                name: name.into(),
                version: String::from("0.1.0"),
            },
            dependencies: HashMap::default(),
        }
    }

    pub async fn read(from: impl AsRef<Path>) -> Result<Option<Self>, YacTomlReadError> {
        let path = from.as_ref().join("Yac.toml");

        if !path.exists() {
            return Ok(None);
        }

        let result = toml::from_str::<Self>(
            &tokio::fs::read_to_string(path).await?
        )?;

        Ok(Some(result))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum YacTomlReadError {
    #[error(transparent)]
    ParseError(#[from] toml::de::Error),

    #[error(transparent)]
    IoError(#[from] tokio::io::Error),
}



#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct YacLock {
    pub package: Vec<(String, Dependency)>,
}

impl YacLock {
    pub fn linearize_here(
        yac_toml: &YacToml, location: impl AsRef<Path>,
    ) -> Result<Self, YacLockLinearizeError> {
        const RECURSION_LIMIT: usize = 256;

        let mut result = vec![];

        Self::linearize_here_recursive(
            yac_toml, location, &mut result, &mut vec![], RECURSION_LIMIT,
        )?;

        Ok(YacLock { package: result })
    }

    fn linearize_here_recursive(
        yac_toml: &YacToml, location: impl AsRef<Path>,
        dependancies: &mut Vec<(String, Dependency)>, used: &mut Vec<String>,
        forward_depth: usize,
    ) -> Result<(), YacLockLinearizeError> {
        if forward_depth == 0 {
            return Err(YacLockLinearizeError::RecursionLimit);
        }

        let location = location.as_ref();

        if used.contains(&yac_toml.package.name) {
            return Ok(());
        }

        used.push(yac_toml.package.name.clone());

        let self_dependancy = Dependency {
            version: yac_toml.package.version.clone(),
            location: Location::path(location.to_str().unwrap().to_owned()),
            r#type: Type::yac,
        };

        for dependancy in yac_toml.dependencies.values() {
            let Location::path(ref location) = dependancy.location else {
                todo!("Location::link is not supported yet");
            };

            let yac_toml = toml::from_str::<YacToml>(
                &std::fs::read_to_string(Path::new(location).join("Yac.toml"))?
            )?;

            Self::linearize_here_recursive(
                &yac_toml, location, dependancies, used, forward_depth - 1,
            )?;
        }

        dependancies.push((yac_toml.package.name.clone(), self_dependancy));
        
        Ok(())
    }
}



#[derive(Debug, thiserror::Error)]
pub enum YacLockLinearizeError {
    #[error(transparent)]
    ParseYacTomlError(#[from] toml::de::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("reached dependancy recursion limit")]
    RecursionLimit,
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deps() {
        const TOML: &str = r#"[package]
name = "test_prj"
version = "0.1.0"

[dependencies]
dep = { version = "0.2.0", type = "cmake", location = { path = "../dep" } }
"#;

        let result = toml::from_str::<YacToml>(TOML).unwrap();

        assert_eq!(result, YacToml {
            package: Package {
                name: "test_prj".into(),
                version: "0.1.0".into(),
            },
            dependencies: HashMap::from([
                ("dep".into(), Dependency {
                    version: "0.2.0".into(),
                    location: Location::path("../dep".into()),
                    r#type: Type::cmake,
                }),
            ]),
        });
    }
}
