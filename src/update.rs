use serde::{Serialize, Deserialize};
use std::path::Path;
use file_hashing::*;
use blake2::*;



pub async fn src_files_updated(release: bool) -> bool {
    const SRC: &str = "src";
    const N_THREADS: usize = 4;

    let mut hash = Blake2s256::new();
    let result = get_hash_folder(SRC, &mut hash, N_THREADS, |_| ()).unwrap();

    let mut update = YacUpdate::read().await.unwrap_or_default().unwrap_or_default();

    if update.last_src_error {
        return true;
    }

    let is_updated = if release {
        update.release_hash != result
    } else {
        update.debug_hash != result
    };

    if is_updated {
        if release {
            update.release_hash = result;
        } else {
            update.debug_hash = result;
        }

        update.write().await.unwrap();
    }

    is_updated
}

pub async fn build_file_updated() -> bool {
    const BUILD: &str = "build.c";

    let mut hash = Blake2s256::new();
    let result = get_hash_file(BUILD, &mut hash).unwrap();

    let mut update = YacUpdate::read().await
        .unwrap_or_default()
        .unwrap_or_default();

    if update.last_build_error {
        return true;
    }

    let is_updated = update.build_hash != result;

    if is_updated {
        update.build_hash = result;
        update.write().await.unwrap();
    }

    is_updated
}

pub async fn yac_toml_updated() -> bool {
    const YAC_TOML: &str = "Yac.toml";

    let mut hash = Blake2s256::new();
    let result = get_hash_file(YAC_TOML, &mut hash).unwrap();

    let mut update = YacUpdate::read().await
        .unwrap_or_default()
        .unwrap_or_default();

    let is_updated = update.build_hash != result;

    if is_updated {
        update.build_hash = result;
        update.write().await.unwrap();
    }

    is_updated
}



#[derive(Serialize, Deserialize, Debug, PartialEq, Hash, Default)]
pub struct YacUpdate {
    pub debug_hash: String,
    pub release_hash: String,
    pub build_hash: String,
    pub yac_toml_hash: String,
    pub last_src_error: bool,
    pub last_build_error: bool,
}

impl YacUpdate {
    pub async fn read() -> Result<Option<Self>, tokio::io::Error> {
        const UPDATE: &str = "target/yac_update.json";

        if !Path::new(UPDATE).exists() {
            return Ok(None);
        }

        Ok(Some(serde_json::from_str::<Self>(
            &tokio::fs::read_to_string(UPDATE).await?,
        )?))
    }

    pub async fn mark_build_error() -> Result<(), tokio::io::Error> {
        let content = Self {
            last_build_error: true,
            ..Self::read().await?.unwrap_or_default()
        };

        content.write().await
    }

    pub async fn mark_src_error() -> Result<(), tokio::io::Error> {
        let content = Self {
            last_src_error: true,
            ..Self::read().await?.unwrap_or_default()
        };

        content.write().await
    }

    pub async fn write(&self) -> Result<(), tokio::io::Error> {
        let ser = serde_json::to_string(self).unwrap();
        tokio::fs::write("target/yac_update.json", ser).await?;

        Ok(())
    }
}