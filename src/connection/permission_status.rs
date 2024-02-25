#[allow(non_camel_case_types)]
#[derive(Debug, Default, Clone, Copy)]
pub enum PermissionStatus {
    #[default]
    InitialConnection = 1,
    //GetPlayerInfo,
    Certified,
    PlayerPermission,
    HostPermission,
    //Debug,
    //Null,
}
