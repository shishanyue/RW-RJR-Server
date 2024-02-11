mod connection_core;
mod core;
mod data;
mod packet_core;
mod server_core;
mod worker_pool_core;
mod worker_pool_manager;
mod connection_manager;


use std::{
    net::SocketAddr,
    path::Path,
    sync::{atomic::Ordering, Arc},
};

use crate::{
    core::{creat_block_runtime, RelayManage},
    data::{COMMAND_HELP, START_INFO},
    server_core::config::*,
};

use connection_manager::ConnectionManager;
use fern::colors::{Color, ColoredLevelConfig};
use log::{info, warn};

use server_core::ServerConfig;

use tokio::{join, net::TcpListener, sync::RwLock, try_join};

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

            let ban_list = Arc::new(RwLock::new(res.banlist));

            let server_data = tokio::spawn(start_server(res.server, ban_list)).await;

            match server_data {
                Ok(server_data) => match server_data {
                    Ok(server_data) => {
                        command_shell(server_data.0, server_data.1, server_data.2).await;
                    }
                    Err(e) => warn!("{}", e),
                },
                Err(e) => warn!("{}", e),
            }
        }
        Err(e) => warn!("{}", e),
    }
}


async fn command_shell(
    _block_runtimes: BlockRuntimes,
    connection_mg: Arc<RwLock<ConnectionManage>>,
    relay_mg: Arc<RwLock<RelayManage>>,
) {
    info!("Server启动成功");
    info!("输入/help获取帮助");
    let std_in = std::io::stdin();
    let mut admin_command = String::new();
    loop {
        std_in.read_line(&mut admin_command).unwrap();
        let mut is_unknown = true;
        if let Some(command) = admin_command.strip_prefix('/') {
            if command.starts_with("help") {
                is_unknown = false;
                info!("{}", COMMAND_HELP);
            } else if command.starts_with("list") {
                if let Some(command) = command.strip_prefix("list ") {
                    let command = command.trim().to_string();
                    if command == "player" {
                        for player in connection_mg.read().await.connections.iter() {
                            let player_name = player
                                .read()
                                .await
                                .player_info
                                .read()
                                .await
                                .player_name
                                .clone();
                            let player_permission_status = player
                                .read()
                                .await
                                .player_info
                                .read()
                                .await
                                .permission_status;
                            println!(
                                "玩家名:{}     权限:{:?}   IP地址:{}",
                                player_name,
                                player_permission_status,
                                player.key()
                            );
                        }
                        is_unknown = false;
                    } else if command == "room" {
                        for room in relay_mg.read().await.room_map.iter() {
                            let room_data = room.relay_room.read().await;
                            println!("id:{}", room_data.id);
                        }
                        is_unknown = false;
                    } else if command == "all_worker" {
                        let receiver_size = connection_mg
                            .read()
                            .await
                            .receiver_size
                            .load(Ordering::Relaxed);
                        let sender_size = connection_mg
                            .read()
                            .await
                            .sender_size
                            .load(Ordering::Relaxed);
                        let processor_size = connection_mg
                            .read()
                            .await
                            .processor_size
                            .load(Ordering::Relaxed);
                        println!(
                            "receiver:{}\nprocessor:{}\nsender:{}",
                            receiver_size, processor_size, sender_size
                        );
                        is_unknown = false;
                    }
                }
            } else if command.starts_with("player") {
                let command = command.trim().to_string();
                if let Some(command) = command.strip_prefix("player ") {
                    if command == "size" {
                        println!("玩家总数:{}", connection_mg.read().await.connections.len());
                        is_unknown = false;
                    }
                }
            } else if command.starts_with("room") {
                let command = command.trim().to_string();
                if let Some(command) = command.strip_prefix("room ") {
                    if command == "size" {
                        println!("房间总数:{}", relay_mg.read().await.room_map.len());
                        is_unknown = false;
                    }
                }
            }
        }
        if is_unknown {
            info!("希腊奶");
        }
        admin_command.clear();
    }
}

async fn start_server(
    server_config: ServerConfig,
    ban_list: Arc<RwLock<Vec<SocketAddr>>>,
) {

    //准备IP地址信息
    let listen_addr = format!("{}{}", "0.0.0.0:", server_config.port);




    let connection_mg = Arc::new(ConnectionManager::new());


    let relay_mg = RelayManage::new().await;

    let listener = TcpListener::bind(&listen_addr).await?;

    tokio::spawn(init_accepter(
        listener,
        connection_mg.clone(),
        ban_list,
        relay_mg.clone(),
    ));
    
}

async fn init_accepter(
    listener: TcpListener,
    connection_mg: Arc<ConnectionManager>,
    ban_list: Arc<RwLock<Vec<SocketAddr>>>,
    relay_mg: Arc<RwLock<RelayManage>>,
) -> anyhow::Result<()> {
    info!("Accepter注册成功");
    loop {
        let (socket, addr) = listener.accept().await?;

        info!("来自{}的新连接", addr);
        
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
