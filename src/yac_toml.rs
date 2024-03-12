use std::{collections::HashMap, path::Path, vec};
use serde::{Serialize, Deserialize};



#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(rename = "type")]
    pub ty: PackageType,
}



#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
    #[default]
    Executable,
    StaticLib,
    DynamicLib,
}



#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dependency {
    pub version: String,
    #[serde(flatten)]
    pub location: Location,
    #[serde(rename = "type", default)]
    pub ty: Type,
}



#[derive(Clone, Default, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    #[default]
    Yac,
    Cmake,
}



#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", untagged)]
pub enum Location {
    Path { path: String },
    Link { link: String },
}



#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct YacToml {
    pub package: Package,
    pub dependencies: HashMap<String, Dependency>,
}

impl YacToml {
    pub fn new(name: impl Into<String>, package_type: PackageType) -> Self {
        Self {
            package: Package {
                name: name.into(),
                version: String::from("0.1.0"),
                ty: package_type,
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
pub struct Target {
    pub yac_toml: YacToml,
    pub description: Dependency,
}



#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct YacLock {
    pub package: Vec<Target>,
}

impl YacLock {
    pub fn linearize_from(
        yac_toml: &YacToml, location: impl AsRef<Path>,
    ) -> Result<Self, YacLockLinearizeError> {
        const RECURSION_LIMIT: usize = 256;

        let mut result = vec![];
        let cwd = std::env::current_dir().unwrap();

        Self::linearize_from_recursive(
            yac_toml.clone(), location, &cwd, &mut result, &mut vec![], RECURSION_LIMIT,
        )?;

        Ok(YacLock { package: result })
    }

    fn linearize_from_recursive(
        yac_toml: YacToml, location: impl AsRef<Path>, cwd: impl AsRef<Path>,
        targets: &mut Vec<Target>, used: &mut Vec<String>,
        forward_depth: usize,
    ) -> Result<(), YacLockLinearizeError> {
        use relative_path::RelativePath;

        if forward_depth == 0 {
            return Err(YacLockLinearizeError::RecursionLimit);
        }

        let location = location.as_ref();
        let cwd = cwd.as_ref();

        if used.contains(&yac_toml.package.name) {
            return Ok(());
        }

        used.push(yac_toml.package.name.clone());

        let pretty_location = RelativePath::from_path(
            &location.to_str().unwrap().replace('\\', "/"),
        ).unwrap().normalize().into_string();

        let self_dependancy = Dependency {
            version: yac_toml.package.version.clone(),
            location: Location::Path { path: pretty_location.clone() },
            ty: Type::Yac,
        };

        for dependancy in yac_toml.dependencies.values() {
            let Location::Path { path: ref relative_location } = dependancy.location else {
                todo!("Location::Link is not supported yet");
            };

            let absolute_location = location.join(relative_location);

            let yac_toml = toml::from_str::<YacToml>(
                &std::fs::read_to_string(absolute_location.join("Yac.toml"))?
            )?;

            Self::linearize_from_recursive(
                yac_toml, &absolute_location, cwd, targets, used, forward_depth - 1,
            )?;
        }

        if !pretty_location.is_empty() {
            targets.push(Target { yac_toml, description: self_dependancy });
        }
        
        Ok(())
    }

    pub async fn read(location: impl AsRef<Path>) -> Result<Option<Self>, YacLockError> {
        let location = location.as_ref().join("Yac.lock");

        if !location.exists() {
            return Ok(None);
        }

        let yac_lock = toml::from_str::<Self>(
            &tokio::fs::read_to_string(&location).await?,
        )?;

        Ok(Some(yac_lock))
    }

    pub async fn write(&self, location: impl AsRef<Path>) -> Result<(), YacLockError> {
        let location = location.as_ref().join("Yac.lock");

        tokio::fs::write(&location, &toml::to_string(self)?).await?;

        Ok(())
    }
}



#[derive(Debug, thiserror::Error)]
pub enum YacLockError {
    #[error(transparent)]
    IoError(#[from] tokio::io::Error),

    #[error(transparent)]
    DeserializeError(#[from] toml::de::Error),

    #[error(transparent)]
    SerializeError(#[from] toml::ser::Error),
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
type = "executable"

[dependencies]
dep = { version = "0.2.0", path = "../dep" }
"#;

        let result = toml::from_str::<YacToml>(TOML).unwrap();

        assert_eq!(result, YacToml {
            package: Package {
                name: "test_prj".into(),
                version: "0.1.0".into(),
                ty: PackageType::Executable,
            },
            dependencies: HashMap::from([
                ("dep".into(), Dependency {
                    version: "0.2.0".into(),
                    location: Location::Path { path: "../dep".into() },
                    ty: Type::Yac,
                }),
            ]),
        });
    }
}
