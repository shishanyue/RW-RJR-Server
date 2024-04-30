#[macro_use]
extern crate lazy_static;

mod connection;
mod connection_manager;
mod core;
mod data;
mod error;
mod packet;
mod relay_manager;
mod server;
mod uplist;
mod worker_pool;
mod command_center;
mod dummy;
mod module;
mod event;



lazy_static! {
    static ref NOW: std::time::Instant = Instant::now();
}

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    connection_manager::By, data::START_INFO, event::init_event_system, packet::super_packet::SuperPacket, server::config::*, uplist::Uplist
};

use connection_manager::ConnectionManager;
use fern::colors::{Color, ColoredLevelConfig};
use log::{info, warn};
use crate::command_center::command_center;

use relay_manager::SharedRelayManager;
use server::ServerConfig;

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
        .expect("get current path error")
        .parent()
        .expect("get current path parent error")
        .join("config.toml");
    let config_dir = binding.as_path();

    info!(
        "当前启动目录:{}\n          配置文件所在目录:{}",
        current_dir,
        config_dir.to_str().unwrap()
    );

    match load_config(config_dir).await {
        Ok(res) => {
            println!("{}", START_INFO);
            info!("加载中.....");
            info!("将从如下配置启动\n{}", res);

            init_event_system().expect("event system init error");

            let shared_connection_mg = start_server(res.server).await.expect("start server error");

            command_center(shared_connection_mg).await;
        }
        Err(e) => {
            warn!("{}", e);
            panic!("{}", e);
        }
    }
}

async fn start_server(server_config: ServerConfig) -> anyhow::Result<Arc<ConnectionManager>> {
    let shared_relay_mg = SharedRelayManager::new(10).await;

    let port_range = server_config.port_range.clone();
    let shared_connection_mg =
        Arc::new(ConnectionManager::new(server_config, shared_relay_mg.clone()).await);

    for port_range in port_range {
        info!("{:?}范围内的Accepter注册成功", port_range);
        for port in port_range.0..port_range.1 {
            let listen_addr = format!("{}{}", "0.0.0.0:", port);
            let listener = TcpListener::bind(&listen_addr).await?;

            tokio::spawn(init_accepter(listener, shared_connection_mg.clone()));
        }
    }
    //准备IP地址信息

    Ok(shared_connection_mg)
}

async fn init_accepter(
    listener: TcpListener,
    connection_mg: Arc<ConnectionManager>,
) -> anyhow::Result<()> {
    loop {
        let new_connection = listener.accept().await?;
        info!("来自{}的新连接", new_connection.1);
        connection_mg.new_connection(new_connection).await;
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
