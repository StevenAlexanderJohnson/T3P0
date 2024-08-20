use crate::{player::PlayerConnection, request::Request, GameState};

#[derive(Debug, Clone)]
pub enum GameMessage {
    Request(Request),
    GameState(GameState),
    PlayerConnection(PlayerConnection),
}
