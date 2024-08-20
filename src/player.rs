use std::sync::Arc;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::GameMessage;

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
    channel: Arc<mpsc::Sender<GameMessage>>,
}

pub trait PlayerConnectionTrait {
    fn new(player: Player, channel: Arc<mpsc::Sender<GameMessage>>) -> Self;
    fn get_player(&self) -> &Player;
    fn get_channel(&self) -> &Arc<mpsc::Sender<GameMessage>>;
}

impl PlayerConnectionTrait for PlayerConnection {
    fn new(player: Player, channel: Arc<mpsc::Sender<GameMessage>>) -> Self {
        PlayerConnection { player, channel }
    }

    fn get_player(&self) -> &Player {
        &self.player
    }

    fn get_channel(&self) -> &Arc<mpsc::Sender<GameMessage>> {
        &self.channel
    }
}
