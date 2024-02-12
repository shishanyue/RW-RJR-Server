use std::sync::{atomic::AtomicU32, Arc};

use tokio::{runtime::Runtime, sync::{
    mpsc,
    Mutex, RwLock,
}};

use crate::{
    connection_core::{permission_status::PermissionStatus, Connection},
    packet_core::{Packet, PacketType},
};

use super::new_worker_pool;

pub type ProcesseorData = ();


pub async fn processor(mut data_receiver: mpsc::Receiver<ProcesseorData>) -> anyhow::Result<()> {
    loop {
        if let Some((con, _packet_sender, packet)) = data_receiver.recv().await {
            //let mut con_lock = con.write().await;
            let packet_type = packet.packet_type;
            let permission = con.read().await.player_info.read().await.permission_status;

            con.write().await.packet = Some(packet.clone());

            match permission {
                PermissionStatus::InitialConnection => match packet_type {
                    PacketType::PREREGISTER_INFO_RECEIVE => {
                        con.write().await.send_relay_server_info().await;

                        con.read().await.player_info.write().await.permission_status =
                            PermissionStatus::Certified;

                        con.write().await.cache_packet = Some(packet);
                        let inspection_data = con.write().await.relay_direct_inspection().await;

                        match inspection_data {
                            Some(data) => match (data.player_name.clone(), data.query_string) {
                                (Some(name), None) => {
                                    con.read().await.player_info.write().await.player_name = name;
                                    con.read()
                                        .await
                                        .connection_info
                                        .write()
                                        .await
                                        .client_version = data.client_version;
                                    con.read()
                                        .await
                                        .connection_info
                                        .write()
                                        .await
                                        .is_beta_version = data.is_beta_version;
                                    con.write()
                                        .await
                                        .send_relay_hall_message(&format!(
                                            "[Relay CN]{} 欢迎使用RJR,这台服务是非官方的Relay房间
This server is CN's unofficial Relay room
输入ID可进入房间，输入new/mods可创建房间
输入/help可以获得更多帮助",
                                            data.player_name.unwrap_or("unkonwn".to_string())
                                        ))
                                        .await;
                                }
                                _ => con.write().await.disconnect().await,
                            },
                            None => con.write().await.disconnect().await,
                        }
                    }
                    PacketType::DISCONNECT => con.write().await.disconnect().await,
                    _ => {}
                },
                PermissionStatus::Certified => match packet_type {
                    PacketType::RELAY_118_117_RETURN => {
                        let all_info = (
                            con.read().await.player_info.clone(),
                            con.read().await.connection_info.clone(),
                        );
                        con.write()
                            .await
                            .send_relay_server_type_reply(all_info)
                            .await
                    }
                    PacketType::HEART_BEAT => {}
                    PacketType::DISCONNECT => con.write().await.disconnect().await,
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
                        con.write().await.send_package_to_host(packet).await;
                    }
                    PacketType::ACCEPT_START_GAME => {
                        con.write().await.send_package_to_host(packet).await;
                    }
                    PacketType::CHAT_RECEIVE => {
                        con.write().await.receive_chat().await;
                    }
                    PacketType::DISCONNECT => {
                        con.write().await.disconnect().await;
                        con.write().await.send_package_to_host(packet).await;
                    },
                    _ => {con.write().await.send_package_to_host(packet).await;}
                },

                PermissionStatus::HostPermission => match packet_type {
                    PacketType::HEART_BEAT => {
                        con.write().await.get_ping_data().await;
                    }
                    PacketType::PACKET_FORWARD_CLIENT_TO => {
                        con.read()
                            .await
                            .room
                            .as_ref()
                            .unwrap()
                            .relay_group_packet_sender
                            .send(packet)
                            .await
                            .unwrap();
                    }
                    PacketType::DISCONNECT => con.write().await.disconnect().await,
                    _ => {}
                },
            };
            //let a = con.clone().lock().await;
        }
    }
}
