#[macro_use]
extern crate lazy_static;

mod connection;
mod connection_manager;
mod core;
mod data;
mod packet;
mod relay_manager;
mod server;
mod worker_pool;

mod rw_engine;

lazy_static! {
    static ref NOW: std::time::Instant = { Instant::now() };
}

use std::{
    net::SocketAddr,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    connection_manager::By, data::START_INFO, packet::super_packet::SuperPacket,
    rw_engine::image::get_image_packet, server::config::*,
};

use connection_manager::ConnectionManager;
use fern::colors::{Color, ColoredLevelConfig};
use log::{info, warn};

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

            let shared_connection_mg = tokio::spawn(start_server(res.server))
                .await
                .expect("start server error")
                .expect("start server error");
            std::thread::sleep(Duration::from_millis(u64::MAX));
            let std_in = std::io::stdin();
            let mut admin_command = String::new();
            loop {
                std_in.read_line(&mut admin_command).unwrap();
                let admin_command = admin_command.trim().to_string();

                for y in 1..200 {
                    let packet =
                        SuperPacket::set_terrain(y as f32 * 20. + 500., y as f32 * 20. + 500., "1")
                            .await;

                    shared_connection_mg
                        .send_packet_to_player_by(By::Addr(admin_command.clone()), packet)
                        .await;
                }

                for y in 1..500 {
                    let packet = SuperPacket::set_terrain(500., y as f32 * 20. + 500., "1").await;

                    shared_connection_mg
                        .send_packet_to_player_by(By::Addr(admin_command.clone()), packet)
                        .await;
                }

                /*
                let packets = get_image_packet(Path::new("1.png")).await.unwrap();

                for p in packets{
                    shared_connection_mg
                        .send_packet_to_player_by(By::Addr(admin_command.clone()), p)
                        .await;
                }

                                for i in 1..200 {
                    let packet =
                        SuperPacket::set_terrain(i as f32 * 20. + 500., i as f32 * 20. + 500.)
                            .await;

                    shared_connection_mg
                        .send_packet_to_player_by(By::Addr(admin_command.clone()), packet)
                        .await;
                }

                 */
            }

            std::thread::sleep(Duration::from_millis(u64::MAX));
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

    let shared_relay_mg = SharedRelayManager::new(10).await;

    let shared_connection_mg =
        Arc::new(ConnectionManager::new(server_config, shared_relay_mg.clone()).await);

    let listener = TcpListener::bind(&listen_addr).await?;

    tokio::spawn(init_accepter(listener, shared_connection_mg.clone()));

    Ok(shared_connection_mg)
}

async fn init_accepter(
    listener: TcpListener,
    connection_mg: Arc<ConnectionManager>,
) -> anyhow::Result<()> {
    info!("Accepter注册成功");
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
