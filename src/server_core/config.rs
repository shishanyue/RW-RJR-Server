use std::{io::ErrorKind, path::Path};

use super::{AllConfig, ServerConfig};

use log::info;
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

pub async fn load_config(path: &Path) -> anyhow::Result<AllConfig> {
    match read_to_string(path).await {
        Ok(s) => {
            info!("配置文件已找到");
            Ok(toml::from_str::<AllConfig>(&s).unwrap())
        }
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                info!("配置文件未找到，已创建新配置文件");
                Ok(save_default_config(path).await.unwrap())
            } else {
                Err(e.into())
            }
        }
    }
}

pub async fn save_default_config(path: &Path) -> anyhow::Result<AllConfig> {
    info!("正在写入配置文件到：{}", path.to_str().unwrap());
    let default = AllConfig::default();
    let mut new_file = File::create(path).await?;
    new_file
        .write_all(toml::to_string(&default).unwrap().as_bytes())
        .await?;
    Ok(default)
}
