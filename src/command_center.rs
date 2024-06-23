use std::{io::Read, sync::Arc};


use crate::{
    connection_manager::ConnectionManager, uplist::Uplist
};

pub async fn command_center(shared_connection_mg: Arc<ConnectionManager>) {
    let _uplsit = Uplist::new(
        "false",
        "RW-Relay-RJR",
        5123,
        1,
        100,
        "RW-RJR",
        "shishanyue",
        "192.168.80.1",
        "开了",
    );


    let mut std_in = std::io::stdin();
    let mut admin_command = String::new();

    
    loop {
        std_in.read_to_string(&mut admin_command).unwrap();
        admin_command.clear();
    }

}
