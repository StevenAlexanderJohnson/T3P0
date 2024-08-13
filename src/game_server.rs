use std::{collections::HashMap, sync::Arc};

use tokio::sync::{
    mpsc::{self},
    Mutex,
};

use crate::{GameState, Player};

#[derive(Debug)]
pub enum GameRequest {
    GetState {
        player_id: Player,
        response: mpsc::Sender<Option<GameState>>,
    },
    UpdateState {
        player_id: Player,
        new_state: GameState,
        response: mpsc::Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    },
    GetPlayerFromQueue {
        response: mpsc::Sender<Option<Player>>,
    },
    AddPlayerToQueue {
        player: Player,
        response: mpsc::Sender<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    },
}

pub struct GameServer {
    queue: Arc<Mutex<Vec<Player>>>,
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

    fn get_player_from_queue(&self) -> impl std::future::Future<Output = Option<Player>> + Send;
    fn insert_player_into_queue(
        &self,
        player: Player,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;

    fn handle_request(&self, game_request: GameRequest) -> impl std::future::Future<Output = ()> + Send;
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

    async fn get_player_from_queue(&self) -> Option<Player> {
        let queue = self.queue.lock().await;
        if queue.is_empty() {
            return None;
        }
        Some(self.queue.lock().await.remove(0))
    }

    async fn insert_player_into_queue(
        &self,
        player: Player,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.queue.lock().await.push(player))
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
                let _ = response.send(self.set_state(player_id, new_state).await);
            }
            GameRequest::GetPlayerFromQueue { response } => {
                let player = self.get_player_from_queue().await;
                let _ = response.send(player);
            }
            GameRequest::AddPlayerToQueue { player, response } => {
                let _ = response.send(self.insert_player_into_queue(player).await);
            }
        };
    }
}
