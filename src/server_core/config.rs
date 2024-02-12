use std::{io::ErrorKind, path::Path};

use super::{AllConfig, ServerConfig};

use log::{info, warn};
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
            Ok(toml::from_str::<AllConfig>(&s).expect("配置文件错误,请检查或删除配置文件并再次运行"))
        }
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                info!("配置文件未找到，已创建新配置文件");
                Ok(save_default_config(path).await.expect("写入默认配置文件失败"))
            } else {
                warn!("{}", e);
                panic!("{}", e);
            }
        }
    }
}

pub async fn save_default_config(path: &Path) -> anyhow::Result<AllConfig> {
    info!("正在写入默认配置文件到:{}", path.to_str().unwrap());
    let default = AllConfig::default();
    let mut new_file = File::create(path).await?;
    new_file
        .write_all(toml::to_string(&default).expect("默认配置转换TOML失败").as_bytes())
        .await?;
    Ok(default)
}
