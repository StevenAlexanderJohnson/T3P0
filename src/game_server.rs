use std::sync::Arc;

use tokio::sync::{oneshot, Mutex};

use crate::{
    player_connection::{PlayerConnection, PlayerConnectionTrait},
    Player,
};

/// GameServerRequest is an enum that represents the different types of requests that can be made to the GameServer.
///
/// # Variants
///
/// * GetPlayerFromQueue - Request a player from the queue.
///   * response - A oneshot channel to send the player to.
/// * AddPlayerToQueue - Add a player to the queue.
///   * player_connection - The player to add to the queue.
/// * RemovePlayerFromQueue - Remove a player from the queue.
///   * player - The player to remove from the queue.
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

/// GameServer is a struct that represents the state of a game server.
///
/// # Fields
///
/// * queue - A Mutex wrapped Vec of PlayerConnections.
pub struct GameServer {
    queue: Arc<Mutex<Vec<PlayerConnection>>>,
}

pub trait GameServerTrait {
    /// Create a new GameServer.
    ///
    /// # Returns
    ///
    /// A new GameServer.
    fn new() -> Self;

    /// Get a player from the queue.
    ///
    /// # Returns
    ///
    /// A future that resolves to an Option<PlayerConnection>.
    fn get_player_from_queue(
        &self,
    ) -> impl std::future::Future<Output = Option<PlayerConnection>> + Send;

    /// Insert a player into the queue.
    ///
    /// # Arguments
    ///
    /// * player_connection - The player to insert into the queue.
    ///
    /// # Returns
    ///
    /// A future that resolves to a Result<(), Box<dyn std::error::Error + Send + Sync>>.
    fn insert_player_into_queue(
        &self,
        player_connection: PlayerConnection,
    ) -> impl std::future::Future<Output = ()> + Send;

    /// Handle a GameServerRequest.
    ///
    /// # Arguments
    ///
    /// * game_request - The request to handle.
    ///
    /// # Returns
    ///
    /// A future that resolves to ().
    ///
    /// # Note
    ///
    /// This function doesn't return a value because channels should be sent within the request as a way to get values out of the GameServer.
    /// This may change in the future.
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

    async fn insert_player_into_queue(&self, player_connection: PlayerConnection) {
        self.queue.lock().await.push(player_connection);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{Player, PlayerTrait};

    #[tokio::test]
    async fn test_new() {
        let game_server = GameServer::new();
        let queue = game_server.queue.lock().await;
        assert!(queue.is_empty());
    }

    #[tokio::test]
    async fn test_get_player_from_queue() {
        let game_server = GameServer::new();
        let player = Arc::new(Player::new());
        let player_connection =
            PlayerConnection::new(player.clone(), Arc::new(tokio::sync::mpsc::channel(1).0));
        game_server
            .insert_player_into_queue(player_connection)
            .await;

        let player_connection = game_server.get_player_from_queue().await.unwrap();
        assert_eq!(player_connection.get_player(), &player);
    }

    #[tokio::test]
    async fn test_get_player_from_empty_queue() {
        let game_server = GameServer::new();
        let player_connection = game_server.get_player_from_queue().await;
        assert!(player_connection.is_none());
    }

    #[tokio::test]
    async fn test_insert_player_into_queue() {
        let game_server = GameServer::new();
        let player = Arc::new(Player::new());
        let player_connection =
            PlayerConnection::new(player.clone(), Arc::new(tokio::sync::mpsc::channel(1).0));
        game_server
            .insert_player_into_queue(player_connection)
            .await;

        let queue = game_server.queue.lock().await;
        assert_eq!(queue.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_request() {
        let game_server = GameServer::new();
        let player = Arc::new(Player::new());
        let player_connection =
            PlayerConnection::new(player.clone(), Arc::new(tokio::sync::mpsc::channel(1).0));
        game_server
            .insert_player_into_queue(player_connection)
            .await;

        let (tx, rx) = oneshot::channel();
        let game_request = GameServerRequest::GetPlayerFromQueue { response: tx };
        game_server.handle_request(game_request).await;

        let player_connection = rx.await.unwrap().unwrap();
        assert_eq!(player_connection.get_player(), &player);
    }

    #[tokio::test]
    async fn test_remove_middle_player() {
        let game_server = GameServer::new();

        let player1 = Arc::new(Player::new());
        let player2 = Arc::new(Player::new());
        let player3 = Arc::new(Player::new());

        let player_connection1 =
            PlayerConnection::new(player1.clone(), Arc::new(tokio::sync::mpsc::channel(1).0));
        let player_connection2 =
            PlayerConnection::new(player2.clone(), Arc::new(tokio::sync::mpsc::channel(1).0));
        let player_connection3 =
            PlayerConnection::new(player3.clone(), Arc::new(tokio::sync::mpsc::channel(1).0));

        game_server.insert_player_into_queue(player_connection1).await;
        game_server.insert_player_into_queue(player_connection2).await;
        game_server.insert_player_into_queue(player_connection3).await;

        let (tx, rx) = oneshot::channel();

        game_server.handle_request(GameServerRequest::RemovePlayerFromQueue {
            player: player2.clone(),
            response: tx,
        }).await;

        let _ = rx.await.unwrap();

        let queue = game_server.queue.lock().await;
        let player = queue.iter().find(|p| p.get_player() == &player2);
        assert_eq!(queue.len(), 2);
        assert!(player.is_none());
    }

    #[tokio::test]
    async fn test_remove_last_player() {
        let game_server = GameServer::new();

        let player1 = Arc::new(Player::new());
        let player2 = Arc::new(Player::new());
        let player3 = Arc::new(Player::new());

        let player_connection1 =
            PlayerConnection::new(player1.clone(), Arc::new(tokio::sync::mpsc::channel(1).0));
        let player_connection2 =
            PlayerConnection::new(player2.clone(), Arc::new(tokio::sync::mpsc::channel(1).0));
        let player_connection3 =
            PlayerConnection::new(player3.clone(), Arc::new(tokio::sync::mpsc::channel(1).0));

        game_server.insert_player_into_queue(player_connection1).await;
        game_server.insert_player_into_queue(player_connection2).await;
        game_server.insert_player_into_queue(player_connection3).await;

        let (tx, rx) = oneshot::channel();

        game_server.handle_request(GameServerRequest::RemovePlayerFromQueue {
            player: player3.clone(),
            response: tx,
        }).await;

        let _ = rx.await.unwrap();

        let queue = game_server.queue.lock().await;
        let player = queue.iter().find(|p| p.get_player() == &player3);
        assert_eq!(queue.len(), 2);
        assert!(player.is_none());
    }
}
