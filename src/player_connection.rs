use std::sync::Arc;

use tokio::sync::mpsc;

use crate::{message::GameMessage, player::Player};

#[derive(Debug, Clone)]
pub struct PlayerConnection {
    player: Arc<Player>,
    channel: Arc<mpsc::Sender<GameMessage>>,
}

pub trait PlayerConnectionTrait {
    fn new(player: Arc<Player>, channel: Arc<mpsc::Sender<GameMessage>>) -> Self;
    fn get_player(&self) -> &Arc<Player>;
    fn get_channel(&self) -> &Arc<mpsc::Sender<GameMessage>>;
}

impl PlayerConnectionTrait for PlayerConnection {
    fn new(player: Arc<Player>, channel: Arc<mpsc::Sender<GameMessage>>) -> Self {
        PlayerConnection { player, channel }
    }

    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }

    fn get_channel(&self) -> &Arc<mpsc::Sender<GameMessage>> {
        &self.channel
    }
}
