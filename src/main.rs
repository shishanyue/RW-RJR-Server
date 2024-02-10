mod connection_core;
mod core;
mod data;
mod packet_core;
mod server_core;
mod worker_pool_core;

use core::BlockRuntime;
use std::{
    net::SocketAddr,
    path::Path,
    sync::{atomic::Ordering, Arc},
};

use crate::{
    core::{creat_block_runtime, ConnectionManage, RelayManage},
    data::{COMMAND_HELP, START_INFO},
    server_core::config::*,
};

use log::{info, warn};

use server_core::ServerConfig;

use tokio::{join, net::TcpListener, sync::RwLock, try_join};

#[tokio::main]
async fn main() {
    init_shell().unwrap();
    // 加载配置文件并初始化终端
    // 完成初始化后开始启动服务器
    //
    info!(
        "当前启动目录：{}",
        std::env::current_dir().unwrap().to_str().unwrap()
    );
    let path = Path::new("config.toml");

    match try_join!(load_config(path),) {
        Ok(res) => {
            println!("{}", START_INFO);
            info!("加载中.....");
            info!("将从如下配置启动\n{}", res.0);

            let ban_list = Arc::new(RwLock::new(res.0.banlist));

            let server_data = tokio::spawn(start_server(res.0.server, ban_list)).await;

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
    };
}

pub type BlockRuntimes = (BlockRuntime, BlockRuntime, BlockRuntime);

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
) -> anyhow::Result<(
    BlockRuntimes,
    Arc<RwLock<ConnectionManage>>,
    Arc<RwLock<RelayManage>>,
)> {
    //准备IP地址信息
    let listen_addr = format!("{}{}", "0.0.0.0:", server_config.port);

    //info!("监听地址：{}", listen_addr);

    let (receiver_block_rt, processor_block_rt, packet_sender_block_rt) = match join!(
        creat_block_runtime(server_config.thread_number),
        creat_block_runtime(server_config.thread_number),
        creat_block_runtime(server_config.thread_number)
    ) {
        (Ok(receiver_block_rt), Ok(processor_block_rt), Ok(packet_sender_block_rt)) => (
            receiver_block_rt,
            processor_block_rt,
            packet_sender_block_rt,
        ),
        _ => {
            panic!()
        }
    };

    let connection_mg = ConnectionManage::new(
        packet_sender_block_rt.clone(),
        receiver_block_rt.clone(),
        processor_block_rt.clone(),
    )
    .await;

    let relay_mg = RelayManage::new().await;

    let listener = TcpListener::bind(&listen_addr).await?;

    tokio::spawn(init_accepter(
        listener,
        connection_mg.clone(),
        ban_list,
        relay_mg.clone(),
    ));
    Ok((
        (
            receiver_block_rt,
            processor_block_rt,
            packet_sender_block_rt,
        ),
        connection_mg,
        relay_mg,
    ))
}

async fn init_accepter(
    listener: TcpListener,
    connection_mg: Arc<RwLock<ConnectionManage>>,
    ban_list: Arc<RwLock<Vec<SocketAddr>>>,
    relay_mg: Arc<RwLock<RelayManage>>,
) -> anyhow::Result<()> {
    //let mut stream_sender = Some(stream_sender);
    //info!("stream_sender={:?}",stream_sender);
    info!("Accepter注册成功");
    loop {
        // Asynchronously wait for an inbound socket.
        let new_con = connection_mg
            .write()
            .await
            .prepare_new_con(relay_mg.clone())
            .await;

        let (socket, addr) = listener.accept().await?;

        if ban_list.read().await.contains(&addr) {
            info!("来自{}的新连接已被黑名单屏蔽", addr);
            continue;
        }
        info!("来自{}的新连接", addr);

        new_con.write().await.bind(addr, socket).await.unwrap();
        connection_mg.write().await.insert_con(addr, new_con).await;
    }
}

fn init_shell() -> anyhow::Result<()> {
    Ok(fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {}] {}",
                humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                record.level(),
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
