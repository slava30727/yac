use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
}



#[derive(Serialize, Deserialize)]
pub struct YacToml {
    pub package: Package,
}

impl YacToml {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            package: Package {
                name: name.into(),
                version: String::from("0.1.0"),
            },
        }
    }
}