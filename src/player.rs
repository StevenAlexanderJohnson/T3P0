use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Player(Uuid);

pub trait PlayerTrait {
    fn new() -> Self;
    fn get_id(&self) -> &Uuid;
    fn from_bytes(bytes: &[u8; 16]) -> Self;
}

impl PlayerTrait for Player {
    fn new() -> Self {
        Player(Uuid::new_v4())
    }

    fn get_id(&self) -> &Uuid {
        &self.0
    }
    
    fn from_bytes(bytes: &[u8; 16]) -> Self {
        Player(*Uuid::from_bytes_ref(bytes))
    }
}