use tokio::sync::mpsc;
use uuid::Uuid;

use crate::GameState;

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

#[derive(Debug, Clone)]
pub struct PlayerConnection {
    player: Player,
    channel: mpsc::Sender<GameState>,
}

pub trait PlayerConnectionTrait {
    fn new(player: Player, channel: mpsc::Sender<GameState>) -> Self;
    fn get_player(&self) -> &Player;
    fn get_channel(&self) -> &mpsc::Sender<GameState>;
}

impl PlayerConnectionTrait for PlayerConnection {
    fn new(player: Player, channel: mpsc::Sender<GameState>) -> Self {
        PlayerConnection { player, channel }
    }

    fn get_player(&self) -> &Player {
        &self.player
    }

    fn get_channel(&self) -> &mpsc::Sender<GameState> {
        &self.channel
    }
}
