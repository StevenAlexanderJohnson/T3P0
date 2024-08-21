use std::sync::Arc;

use tokio::sync::{oneshot, Mutex};

use crate::{
    player::{PlayerConnection, PlayerConnectionTrait},
    Player,
};

#[derive(Debug)]
pub enum GameServerRequest {
    GetPlayerFromQueue {
        response: oneshot::Sender<Option<PlayerConnection>>,
    },
    AddPlayerToQueue {
        player_connection: PlayerConnection,
    },
    RemovePlayerFromQueue {
        player: Arc<Player>,
        response: oneshot::Sender<()>,
    },
}

pub struct GameServer {
    queue: Arc<Mutex<Vec<PlayerConnection>>>,
}

pub trait GameServerTrait {
    fn new() -> Self;

    fn get_player_from_queue(
        &self,
    ) -> impl std::future::Future<Output = Option<PlayerConnection>> + Send;
    fn insert_player_into_queue(
        &self,
        player_connection: PlayerConnection,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send;

    fn handle_request(
        &self,
        game_request: GameServerRequest,
    ) -> impl std::future::Future<Output = ()> + Send;
}

impl GameServerTrait for GameServer {
    fn new() -> Self {
        GameServer {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn get_player_from_queue(&self) -> Option<PlayerConnection> {
        let mut queue = self.queue.lock().await;
        if queue.is_empty() {
            return None;
        }
        let player = queue.remove(0);
        Some(player)
    }

    async fn insert_player_into_queue(
        &self,
        player_connection: PlayerConnection,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.queue.lock().await.push(player_connection);
        Ok(())
    }

    async fn handle_request(&self, game_request: GameServerRequest) {
        match game_request {
            GameServerRequest::GetPlayerFromQueue { response } => {
                let player = self.get_player_from_queue().await;
                let _ = response.send(player);
            }
            GameServerRequest::AddPlayerToQueue { player_connection } => {
                let _ = self.insert_player_into_queue(player_connection).await;
            }
            GameServerRequest::RemovePlayerFromQueue { player, response } => {
                let mut queue = self.queue.lock().await;
                let index = queue.iter().position(|p| p.get_player() == &player);
                if let Some(index) = index {
                    let player = queue.remove(index);
                    drop(player);
                }

                let _ = response.send(());
            }
        };
    }
}
