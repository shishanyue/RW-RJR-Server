





#[derive(Debug)]
pub struct RelayDirectInspection {
    pub client_version: u32,
    pub is_beta_version: bool,
    pub query_string: Option<String>,
    pub player_name: Option<String>,
}
#[derive(Debug)]
pub struct CustomRelayData {
    pub max_player_size: i32,
    pub max_unit_size: u32,
    pub income: f32,
    pub uplist: bool,
    pub mods: bool,
    pub beta_game_version: bool,
    pub version: u32,
}

impl CustomRelayData {
    pub fn new(mods: bool, uplist: bool, beta_game_version: bool, version: u32) -> Self {
        Self {
            max_player_size: -1,
            max_unit_size: 300,
            income: 1.0,
            mods,
            uplist,
            version,
            beta_game_version
        }
    }
}

//(u32,bool,Option<String>,Option<String>);
