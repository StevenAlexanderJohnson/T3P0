use crate::{player_connection::PlayerConnection, GameState};

#[derive(Debug, Clone)]
pub enum GameMessage {
    GameState(GameState),
    PlayerConnection(PlayerConnection),
}
