mod connection_core;
mod core;
mod data;
mod packet_core;
mod server_core;
mod worker_pool_core;
use core::BlockRuntime;
use std::{net::SocketAddr, path::Path, sync::Arc, time::Duration};

use crate::{
    core::{creat_block_runtime, ConnectionManage, RelayManage},
    data::START_INFO,
    server_core::config::*,
};

use log::{info, warn};

use server_core::ServerConfig;

use tokio::{
    join,
    net::TcpListener,
    sync::{Mutex, RwLock},
    try_join,
};

#[tokio::main]
async fn main() {
    // 加载配置文件并初始化终端
    // 完成初始化后开始启动服务器
    //
    let path = Path::new("config.toml");

    match try_join!(load_config(path), init_shell()) {
        Ok(res) => {
            println!("{}", START_INFO);
            info!("加载中.....");
            info!("将从如下配置启动\n{}", res.0);

            let ban_list = Arc::new(RwLock::new(res.0.banlist));

            let runtimes = tokio::spawn(start_server(res.0.server, ban_list)).await;

            match runtimes {
                Ok(runtimes) => match runtimes {
                    Ok(runtimes) => {
                        command_shell(runtimes).await;
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

async fn command_shell(block_runtimes: BlockRuntimes) {
    info!("Server启动成功");
    loop {}
}

async fn start_server(
    server_config: ServerConfig,
    ban_list: Arc<RwLock<Vec<SocketAddr>>>,
) -> anyhow::Result<BlockRuntimes> {
    //准备IP地址信息
    let listen_addr = format!("{}{}", "0.0.0.0:", server_config.port);

    //info!("监听地址：{}", listen_addr);

    let (receiver_block_rt, processor_block_rt, packet_sender_block_rt) = match join!(
        creat_block_runtime(),
        creat_block_runtime(),
        creat_block_runtime()
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
        connection_mg,
        ban_list,
        relay_mg.clone(),
    ));
    Ok((
        receiver_block_rt,
        processor_block_rt,
        packet_sender_block_rt,
    ))
}

async fn init_accepter(
    listener: TcpListener,
    connection_mg: Arc<RwLock<ConnectionManage>>,
    ban_list: Arc<RwLock<Vec<SocketAddr>>>,
    relay_mg: Arc<Mutex<RelayManage>>,
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

async fn init_shell() -> anyhow::Result<()> {
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
