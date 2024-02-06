use std::{io::ErrorKind, path::Path};

use super::{AllConfig, GameConfig, ServerConfig, UplistApi};

use tokio::{
    fs::{read_to_string, File},
    io::AsyncWriteExt,
};

use toml;

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 5123,
            thread_number: 30,
            enable_web: false,
        }
    }
}
impl Default for UplistApi {
    fn default() -> Self {
        Self {}
    }
}
impl Default for GameConfig {
    fn default() -> Self {
        Self {}
    }
}

pub async fn load_config(path: &Path) -> anyhow::Result<AllConfig> {
    match read_to_string(path).await {
        Ok(s) => {
            Ok(toml::from_str::<AllConfig>(&s).unwrap_or(save_default_config(path).await.unwrap()))
        }
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                Ok(save_default_config(path).await.unwrap())
            } else {
                Err(e.into())
            }
        }
    }
}

pub async fn save_default_config(path: &Path) -> anyhow::Result<AllConfig> {
    let default = AllConfig::default();
    let mut new_file = File::create(path).await?;
    new_file
        .write_all(toml::to_string(&default).unwrap().as_bytes())
        .await?;
    Ok(default)
}
