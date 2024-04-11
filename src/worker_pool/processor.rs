use std::sync::{atomic::{AtomicI64, AtomicU32, Ordering}, Arc};


use tokio::sync::SemaphorePermit;

use crate::{
    connection::{permission_status::PermissionStatus, shared_connection::SharedConnection},
    packet::{Packet, PacketType},
};

pub type ProcesseorData = (Arc<SharedConnection>, Packet);

pub async fn processor(data: async_channel::Receiver<(ProcesseorData,Arc<AtomicI64>)>) -> anyhow::Result<()> {
    loop {
        match data.recv().await {
            Ok(((shared_con, packet),permit)) => {
                permit.fetch_add(1, Ordering::Relaxed);
                let packet_type = packet.packet_type;
                let player_info_arc = shared_con.shared_data.player_info.clone();
                let connection_info_arc = shared_con.shared_data.connection_info.clone();
                let permission = *player_info_arc.permission_status.read().unwrap();

                shared_con.set_packet(packet.clone()).await;

                match permission {
                    PermissionStatus::InitialConnection => match packet_type {
                        PacketType::PREREGISTER_INFO_RECEIVE => {
                            shared_con.send_relay_server_info().await;

                            *player_info_arc.permission_status.write().unwrap() =
                                PermissionStatus::Certified;

                            shared_con.set_cache_packet(packet).await;

                            let inspection_data = shared_con.relay_direct_inspection().await;

                            if let Some(data) = inspection_data {
                                if let (Some(name), None) =
                                    (data.player_name.clone(), data.query_string)
                                {
                                    *player_info_arc.player_name.write().unwrap() = name;

                                    connection_info_arc
                                        .client_version
                                        .store(data.client_version, Ordering::Relaxed);
                                    connection_info_arc
                                        .is_beta_version
                                        .store(data.is_beta_version, Ordering::Relaxed);
                                    shared_con.send_relay_hall_message(&format!(
                                "[Relay CN]{} 欢迎使用RJR,这台服务是非官方的Relay房间\nThis server is CN's unofficial Relay room\n输入ID可进入房间,输入new/mods可创建房间\n输入/help可以获得更多帮助",
                                data.player_name.unwrap_or("unkonwn".to_string())
                                )).await;
                                } else {
                                    todo!()
                                }
                            } else {
                                shared_con.disconnect().await;
                            }
                        }
                        PacketType::DISCONNECT => shared_con.disconnect().await,
                        _ => {}
                    },
                    PermissionStatus::Certified => match packet_type {
                        PacketType::RELAY_118_117_RETURN => {
                            shared_con.send_relay_server_type_reply().await
                        }
                        PacketType::HEART_BEAT => {}
                        PacketType::DISCONNECT => shared_con.disconnect().await,
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
        
        
                        PacketType::REGISTER_PLAYER => 
                            shared_con.send_packet_to_host(packet).await,
                        
                        PacketType::ACCEPT_START_GAME => 
                            shared_con.send_packet_to_host(packet).await,
                        
                        PacketType::CHAT_RECEIVE => 
                            shared_con.send_packet_to_host(packet).await,
                        
                        PacketType::DISCONNECT => {
                            shared_con.disconnect().await;
                            shared_con.send_packet_to_host(packet).await
                        },
                        _ => shared_con.send_packet_to_host(packet).await,
                    },

                    PermissionStatus::HostPermission => match packet_type {
                        PacketType::HEART_BEAT => shared_con.get_ping_data().await,
                        PacketType::PACKET_FORWARD_CLIENT_TO => {
                            shared_con.send_packet_to_others(packet).await;
                        }
                        PacketType::DISCONNECT => shared_con.disconnect().await,
                        _ => {}
                    },
                };
                permit.fetch_add(-1, Ordering::Relaxed);
            }
            Err(_) => continue,
        }
    }
}
