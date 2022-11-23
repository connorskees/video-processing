use crate::Uuid;

#[derive(Debug)]
pub enum SeiMessage {
    UserDataUnregistered(UserDataUnregistered),
}

#[derive(Debug)]
pub struct UserDataUnregistered {
    pub uuid: Uuid,
    pub payload: Vec<u8>,
}
