use std::sync::Arc;


use tokio::sync::{
        mpsc::{self}, Mutex, RwLock,
    };

use crate::{
    connection_core::{permission_status::PermissionStatus, Connection},
    core::BlockRuntime,
    packet_core::{Packet, PacketType},
};

use super::new_worker_pool;

pub type ProcesseorData = (Arc<RwLock<Connection>>, mpsc::Sender<Packet>, Packet);

pub async fn init_processor_sorter(processor_rt: BlockRuntime) -> mpsc::Sender<ProcesseorData> {
    let (packet_process_sender, mut packet_process_receiver) = mpsc::channel::<ProcesseorData>(10);

    let processor_pool = new_worker_pool(
        100,
        move |p_receiver, _| Box::pin(processor(p_receiver)),
        processor_rt,
        (),
    )
    .await;

    tokio::spawn(async move {
        let processor_pool = Arc::new(Mutex::new(processor_pool));
        //let mut receiver = Some(tcp_receiver);
        loop {
            let processor = processor_pool.lock().await.get_free_worker().await;
            match packet_process_receiver.recv().await {
                Some(data) => {
                    let temporary_processor_pool = processor_pool.clone();
                    tokio::spawn(async move {
                        //let processor_pool = processor_pool.clone();
                        processor.send(data).await.unwrap();
                        temporary_processor_pool
                            .lock()
                            .await
                            .push_free_worker(processor.clone())
                            .await;
                    });
                }
                None => todo!(),
            }
        }
    });

    packet_process_sender
}

pub async fn processor(mut data_receiver: mpsc::Receiver<ProcesseorData>) -> anyhow::Result<()> {
    loop {
        match data_receiver.recv().await {
            Some((con, _packet_sender, packet)) => {
                let mut con_lock = con.write().await;
                let packet_type = packet.packet_type;
                let permission = con_lock.player_info.read().await.permission_status;

                con_lock.packet = Some(packet.clone());

                match permission {
                    PermissionStatus::InitialConnection => match packet_type {
                        PacketType::PREREGISTER_INFO_RECEIVE => {
                            con_lock.send_relay_server_info().await;

                            con_lock.player_info.write().await.permission_status =
                                PermissionStatus::Certified;

                            con_lock.cache_packet = Some(packet);
                            let inspection_data = con_lock.relay_direct_inspection().await;

                            match inspection_data {
                                Some(data) => match (data.player_name.clone(), data.query_string) {
                                    (Some(name), None) => {
                                        con_lock.player_info.write().await.player_name = name;
                                        con_lock.connection_info.write().await.client_version =
                                            data.client_version;
                                        con_lock.connection_info.write().await.is_beta_version =
                                            data.is_beta_version;
                                        con_lock
                                            .send_relay_hall_message(&format!(
                                                "{},欢迎来到RJR",
                                                data.player_name.unwrap_or("unkonwn".to_string())
                                            ))
                                            .await;
                                    }
                                    _ => todo!(),
                                },
                                None => todo!(),
                            }
                        }
                        PacketType::DISCONNECT => {
                            con_lock.disconnect().await;
                        }
                        _ => {}
                    },
                    PermissionStatus::Certified => match packet_type {
                        PacketType::RELAY_118_117_RETURN => {
                            let all_info = (
                                con_lock.player_info.clone(),
                                con_lock.connection_info.clone(),
                            );
                            con_lock.send_relay_server_type_reply(all_info).await;
                        }
                        PacketType::HEART_BEAT => {}
                        _ => {}
                    },
                    PermissionStatus::PlayerPermission => match packet_type {
                        // USERS (NON-HOST) SHOULD NOT SEND RELAY PACKETS
                        |PacketType::RELAY_117|PacketType::RELAY_118_117_RETURN|PacketType::RELAY_POW|PacketType::RELAY_POW_RECEIVE|PacketType::RELAY_VERSION_INFO|PacketType::RELAY_BECOME_SERVER|PacketType::FORWARD_CLIENT_ADD|PacketType::FORWARD_CLIENT_REMOVE|PacketType::PACKET_FORWARD_CLIENT_FROM|PacketType::PACKET_FORWARD_CLIENT_TO|PacketType::PACKET_FORWARD_CLIENT_TO_REPEATED
                        // 防止假冒
                        |PacketType::TICK|PacketType::SYNC|PacketType::SERVER_INFO|PacketType::HEART_BEAT|PacketType::START_GAME|PacketType::RETURN_TO_BATTLEROOM|PacketType::CHAT|PacketType::KICK|PacketType::PACKET_RECONNECT_TO
                        //每个玩家不一样, 不需要处理/缓存|
                        |PacketType::PASSWD_ERROR|PacketType::TEAM_LIST|PacketType::PREREGISTER_INFO
                        // Nobody dealt with it
                        |PacketType::PACKET_DOWNLOAD_PENDING
                        // Refusal to Process
                        |PacketType::EMPTYP_ACKAGE|PacketType::NOT_RESOLVED => {}


                        PacketType::REGISTER_PLAYER => {
                            con_lock.send_package_to_host(packet).await;
                        }
                        PacketType::ACCEPT_START_GAME => {
                            con_lock.send_package_to_host(packet).await;
                        }
                        PacketType::CHAT_RECEIVE => {
                            con_lock.receive_chat().await;
                        }
                        _ => {con_lock.send_package_to_host(packet).await;}
                    },

                    PermissionStatus::HostPermission => {
                        match packet_type {
                            PacketType::HEART_BEAT => {
                                con_lock.get_ping_data().await;
                            }
                            PacketType::PACKET_FORWARD_CLIENT_TO => {
                                con_lock
                                    .room
                                    .as_ref()
                                    .unwrap()
                                    .relay_group_packet_sender
                                    .send(packet)
                                    .await
                                    .unwrap();
                            }
                            _ => {}
                        }
                    }
                };
                //let a = con.clone().lock().await;
            }
            None => todo!(),
        }
    }
}
