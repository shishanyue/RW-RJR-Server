mod connection_core;
mod connection_manager;
mod core;
mod data;
mod packet_core;
mod server_core;
mod worker_pool_core;
use std::{
    net::SocketAddr,
    path::Path,
    sync::{atomic::Ordering, Arc},
};

use crate::{
    core::RelayManage,
    data::{COMMAND_HELP, START_INFO},
    server_core::config::*,
};

use connection_manager::ConnectionManager;
use fern::colors::{Color, ColoredLevelConfig};
use log::{info, warn};

use server_core::ServerConfig;

use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    init_shell().unwrap();
    // 加载配置文件并初始化终端
    // 完成初始化后开始启动服务器
    //

    let binding = std::env::current_dir().unwrap();
    let current_dir = binding.to_str().unwrap();
    let binding = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("config.toml");
    let config_dir = binding.as_path();

    info!(
        "当前启动目录:{}\n\t\t\t\t配置文件所在目录:{}",
        current_dir,
        config_dir.to_str().unwrap()
    );

    match load_config(config_dir).await {
        Ok(res) => {
            println!("{}", START_INFO);
            info!("加载中.....");
            info!("将从如下配置启动\n{}", res);

            tokio::spawn(start_server(res.server)).await;

            loop {
                
            }
        }
        Err(e) => {
            warn!("{}", e);
            panic!("{}", e);
        }
    }
}

async fn start_server(server_config: ServerConfig) -> anyhow::Result<Arc<ConnectionManager>> {
    //准备IP地址信息
    let listen_addr = format!("{}{}", "0.0.0.0:", server_config.port);

    let connection_mg = Arc::new(ConnectionManager::new(server_config).await);

    let listener = TcpListener::bind(&listen_addr).await?;

    tokio::spawn(init_accepter(listener, connection_mg.clone()));

    Ok(connection_mg)
}

async fn init_accepter(
    listener: TcpListener,
    connection_mg: Arc<ConnectionManager>,
) -> anyhow::Result<()> {
    info!("Accepter注册成功");
    loop {
        let new_connection = listener.accept().await?;
        info!("来自{}的新连接", new_connection.1);
        connection_mg
            .new_connection_sender
            .send(new_connection)
            .await
            .expect("添加新连接发送失败");
    }
}

fn init_shell() -> anyhow::Result<()> {
    let mut colors = ColoredLevelConfig::new()
        // use builder methods
        .info(Color::Green);
    // or access raw fields
    colors.warn = Color::Magenta;

    Ok(fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                colors.color(record.level()),
                //record.target(),
                message
            ))
        })
        // Add blanket level filter -
        .level(log::LevelFilter::Debug)
        // - and per-module overrides
        .level_for("hyper", log::LevelFilter::Info)
        // Output to stdout, files, and other Dispatch configurations
        .chain(std::io::stdout())
        .chain(fern::log_file("output.log")?)
        // Apply globally
        .apply()?)
}
