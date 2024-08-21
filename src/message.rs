use crate::{player::PlayerConnection, GameState};

#[derive(Debug, Clone)]
pub enum GameMessage {
    GameState(GameState),
    PlayerConnection(PlayerConnection),
}
