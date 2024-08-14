use std::{collections::HashMap, sync::Arc};

use tokio::sync::{oneshot, Mutex};

use crate::{GameState, Player};

#[derive(Debug)]
pub enum GameRequest {
    GetState {
        player_id: Player,
        response: oneshot::Sender<Option<GameState>>,
    },
    UpdateState {
        player_id: Player,
        new_state: GameState,
        response: oneshot::Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    },
    GetPlayerFromQueue {
        response: oneshot::Sender<Option<(Player, oneshot::Sender<Player>)>>,
    },
    AddPlayerToQueue {
        player: Player,
        response: oneshot::Sender<Player>,
    },
}

pub struct GameServer {
    queue: Arc<Mutex<Vec<(Player, oneshot::Sender<Player>)>>>,
    game_state_map_clone: Arc<Mutex<HashMap<Player, GameState>>>,
}

pub trait GameServerTrait {
    fn new() -> Self;
    fn get_state(
        &self,
        player_id: Player,
    ) -> impl std::future::Future<Output = Option<GameState>> + Send;
    fn set_state(
        &self,
        player_id: Player,
        new_state: GameState,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;

    async fn get_player_from_queue(&self) -> Option<(Player, oneshot::Sender<Player>)>;
    async fn insert_player_into_queue(
        &self,
        player: Player,
        response_channel: oneshot::Sender<Player>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    fn handle_request(
        &self,
        game_request: GameRequest,
    ) -> impl std::future::Future<Output = ()> + Send;
}

impl GameServerTrait for GameServer {
    fn new() -> Self {
        GameServer {
            queue: Arc::new(Mutex::new(Vec::new())),
            game_state_map_clone: Arc::new(Mutex::new(HashMap::<Player, GameState>::new())),
        }
    }

    async fn get_state(&self, player_id: Player) -> Option<GameState> {
        self.game_state_map_clone
            .lock()
            .await
            .get(&player_id)
            .cloned()
    }

    async fn set_state(
        &self,
        player_id: Player,
        new_state: GameState,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.game_state_map_clone
            .lock()
            .await
            .insert(player_id, new_state);

        Ok(())
    }

    async fn get_player_from_queue(&self) -> Option<(Player, oneshot::Sender<Player>)> {
        let mut queue = self.queue.lock().await;
        if queue.is_empty() {
            return None;
        }
        let player = queue.remove(0);
        Some(player)
    }

    async fn insert_player_into_queue(
        &self,
        player: Player,
        response_channel: oneshot::Sender<Player>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.queue.lock().await.push((player, response_channel));
        Ok(())
    }

    async fn handle_request(&self, game_request: GameRequest) {
        match game_request {
            GameRequest::GetState {
                player_id,
                response,
            } => {
                let state = self.get_state(player_id).await;
                let _ = response.send(state);
            }
            GameRequest::UpdateState {
                player_id,
                new_state,
                response,
            } => {
                let result = self.set_state(player_id, new_state).await;
                let _ = response.send(result);
            }
            GameRequest::GetPlayerFromQueue { response } => {
                let player = self.get_player_from_queue().await;
                let _ = response.send(player);
            }
            GameRequest::AddPlayerToQueue { player, response } => {
                let _ = self.insert_player_into_queue(player, response).await;
            }
        };
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::{GameStateTrait, PlayerTrait};

//     use super::*;

//     #[tokio::test]
//     async fn test_add_player_to_queue() {
//         let game_server = GameServer::new();
//         // create a tokio::sync::oneshot channel
//         let (tx, rx) = oneshot::channel::<Result<(), Box<dyn std::error::Error + Send + Sync>>>();

//         let player = Player::new();
//         game_server
//             .handle_request(GameRequest::AddPlayerToQueue {
//                 player: player.clone(),
//                 response: tx,
//             })
//             .await;

//         let result = rx.await.unwrap();

//         assert!(result.is_ok());
//     }

//     #[tokio::test]
//     async fn test_get_player_from_queue() {
//         let game_server = GameServer::new();
//         let player = Player::new();
//         let result = game_server.insert_player_into_queue(player.clone()).await;
//         assert!(result.is_ok());

//         let player_from_queue = game_server
//             .get_player_from_queue()
//             .await
//             .expect("Player not found in queue");

//         assert_eq!(player, player_from_queue);
//     }

//     #[tokio::test]
//     async fn test_get_player_empty() {
//         let game_server = GameServer::new();
//         let player = Player::new();
//         let result = game_server.insert_player_into_queue(player.clone()).await;
//         assert!(result.is_ok());

//         let player_from_queue = game_server.get_player_from_queue().await;
//         assert!(player_from_queue.is_some());
//         assert_eq!(player, player_from_queue.unwrap());

//         let player_from_queue = game_server.get_player_from_queue().await;
//         assert!(player_from_queue.is_none());
//     }

//     #[tokio::test]
//     async fn test_get_state() {
//         let game_server = GameServer::new();
//         let player = Player::new();
//         let state = GameState::new(Some(player.clone()), None);
//         let result = game_server.set_state(player.clone(), state.clone()).await;
//         assert!(result.is_ok());

//         let state_from_server = game_server.get_state(player.clone()).await;
//         assert!(state_from_server.is_some());
//         assert_eq!(state.to_request(), state_from_server.unwrap().to_request());
//     }

//     #[tokio::test]
//     async fn test_get_state_empty() {
//         let game_server = GameServer::new();
//         let player = Player::new();
//         let state = GameState::new(Some(player.clone()), None);
//         let result = game_server.set_state(player.clone(), state.clone()).await;
//         assert!(result.is_ok());

//         let state_from_server = game_server.get_state(player.clone()).await;
//         assert!(state_from_server.is_some());
//         assert_eq!(state.to_request(), state_from_server.unwrap().to_request());

//         let state_from_server = game_server.get_state(Player::new()).await;
//         assert!(state_from_server.is_none());
//     }

//     #[tokio::test]
//     async fn test_set_state() {
//         let game_server = GameServer::new();
//         let player = Player::new();
//         let state = GameState::new(Some(player.clone()), None);
//         let result = game_server.set_state(player.clone(), state.clone()).await;
//         assert!(result.is_ok());

//         let state_from_server = game_server.get_state(player.clone()).await;
//         assert!(state_from_server.is_some());
//         assert_eq!(state.to_request(), state_from_server.unwrap().to_request());
//     }

//     #[tokio::test]
//     async fn test_set_state_overwrite() {
//         let game_server = GameServer::new();
//         let player = Player::new();
//         let state = GameState::new(Some(player.clone()), None);
//         let result = game_server.set_state(player.clone(), state.clone()).await;
//         assert!(result.is_ok());

//         let state_from_server = game_server.get_state(player.clone()).await;
//         assert!(state_from_server.is_some());
//         assert_eq!(state.to_request(), state_from_server.unwrap().to_request());

//         let state = GameState::new(Some(player.clone()), None);
//         let result = game_server.set_state(player.clone(), state.clone()).await;
//         assert!(result.is_ok());

//         let state_from_server = game_server.get_state(player.clone()).await;
//         assert!(state_from_server.is_some());
//         assert_eq!(state.to_request(), state_from_server.unwrap().to_request());
//     }
// }
