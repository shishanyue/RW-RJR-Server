use std::{sync::Arc, time::Duration};

use chrono::{Timelike, Utc};
use log::info;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::Client;
use tokio::task::JoinHandle;
use uuid::Uuid;


type Token = String;
type ServerUuid = String;
static UPLIST_URL: [&str; 2] = [
    "http://gs1.corrodinggames.com/masterserver/1.4/interface",
    "http://gs4.corrodinggames.com/masterserver/1.4/interface",
];
#[derive(Clone)]
pub struct UplistData {
    server_uuid: ServerUuid,
    game_name: String,
    time: u32,
    token: Token,
    passwd: String,
    port: usize,
    player_size: usize,
    player_max_size: usize,
    game_map: String,
    created_by: String,
    private_ip: String,
    game_status: String,
}

pub struct Uplist {
    data: Arc<UplistData>,
    handle: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
}

impl Uplist {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        passwd: &str,
        game_name: &str,
        port: usize,
        player_size: usize,
        player_max_size: usize,
        game_map: &str,
        created_by: &str,
        private_ip: &str,
        game_status: &str,
    ) -> Self {
        let token = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(40)
            .map(char::from)
            .collect();

        let now = Utc::now();

        let uplist_data = Arc::new(UplistData {
            server_uuid: format!("u_{}", Uuid::new_v4()),
            token,
            game_name: game_name.to_string(),
            time: now.second(),
            passwd: passwd.to_string(),
            player_max_size,
            player_size,
            port,
            created_by: created_by.to_string(),
            private_ip: private_ip.to_string(),
            game_map: game_map.to_string(),
            game_status: game_status.to_string(),
        });
        Uplist {
            data: uplist_data.clone(),
            handle: tokio::spawn(uplist_fn(uplist_data)),
        }
    }
}

async fn uplist_fn(
    uplist_data: Arc<UplistData>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut client = reqwest::Client::new();
    uplist_add(&mut client, &uplist_data).await?;

    let update_body = format!("action=update&id={}&private_token={}&password_required={}&created_by={}&private_ip=10.0.0.1&port_number={}&game_map={}&game_mode=skirmishMap&game_status={}&player_count={}&max_player_count={}",uplist_data.server_uuid,uplist_data.token,uplist_data.passwd,uplist_data.created_by,uplist_data.port,uplist_data.game_map,uplist_data.game_status,uplist_data.player_size,uplist_data.player_max_size);

    loop {
        uplist_update(&mut client, &update_body).await?;
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

async fn uplist_add(
    client: &mut Client,
    uplist_data: &Arc<UplistData>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let add_body = format!("action=add&user_id={}&game_name={}&_1={}&tx2={}&tx3={}&game_version=151&game_version_string=1.14&game_version_beta=false&private_token={}&private_token_2={}&confirm={}&password_required={}&created_by={}&private_ip={}&port_number={}&game_map={}&game_mode=skirmishMap&game_status=battleroom&player_count={}&max_player_count={}", 
    uplist_data.server_uuid,uplist_data.game_name,uplist_data.time,
    format!("{:x}",md5::compute(&sha256::digest(format!("SHA256_{}",&uplist_data.server_uuid[..=5]))[..=4])).to_ascii_uppercase(),
    format!("{:x}",md5::compute(&sha256::digest(format!("SHA256_{}{}",&uplist_data.server_uuid[..=5],uplist_data.time))[..=4])).to_ascii_uppercase(),
    uplist_data.token,
    format_args!("{:x}",md5::compute(md5::compute(&uplist_data.token).0)),
    format_args!("{:x}",md5::compute(format!("a{:x}",&md5::compute(&uplist_data.token)))),
    uplist_data.passwd,
    uplist_data.created_by,
    uplist_data.private_ip,
    uplist_data.port,
    uplist_data.game_map,
    uplist_data.player_size,
    uplist_data.player_max_size
    );

    for url in UPLIST_URL {
        match client
            .post(url)
            .timeout(Duration::from_secs(1))
            .body(add_body.clone())
            .send()
            .await
        {
            Ok(_) => {
                info!("uplist add 从{}添加列表成功", url);
            }
            Err(e) => {
                if e.is_timeout() {
                    continue;
                } else {
                    return Err(Box::new(e));
                }
            }
        }
    }
    Ok(())
}

async fn uplist_update(
    client: &mut Client,
    update_body: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for url in UPLIST_URL {
        match client
            .post(url)
            .timeout(Duration::from_secs(1))
            .body(update_body.to_string())
            .send()
            .await
        {
            Ok(_) => {}
            Err(e) => {
                if e.is_timeout() {
                    continue;
                } else {
                    return Err(Box::new(e));
                }
            }
        }
    }
    Ok(())
}
