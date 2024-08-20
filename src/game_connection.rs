use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, oneshot},
    time::{interval, Duration},
};

use crate::{
    player::{PlayerConnection, PlayerConnectionTrait},
    request::Request,
    DataRequest, GameMessage, GameServerRequest, GameState, GameStateTrait, Player, PlayerTrait,
};

/// A struct that represents a connection to a game server.
/// The connection is used to send and receive messages to and from the server.
///
/// # Fields
///
/// * `player` - The player that is connected to the server.
/// * `connection` - The connection to the server.
/// * `tx` - The sending channel to send requests to the main thread.
pub struct GameConnection {
    player: Player,
    connection: TcpStream,
    game_state: GameState,
    tx: mpsc::Sender<GameServerRequest>,
    opponent_sender: Option<Arc<mpsc::Sender<GameMessage>>>,
    opponent_receiver: mpsc::Receiver<GameMessage>,
}

pub trait GameConnectionTrait {
    /// Create a new GameConnection
    ///
    /// # Arguments
    ///
    /// * `player` - The player that is connected to the server.
    /// * `connection` - The connection to the server.
    /// * `tx` - The sending channel to send requests to the main thread.
    fn new(connection: TcpStream, tx: mpsc::Sender<GameServerRequest>) -> Self;
    /// Handles the handshake between the client and the server.
    fn handshake(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
    fn get_opponent_and_initialize_state(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
    /// Handles the request from the client.
    fn handle_request(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
    fn cleanup(
        &mut self,
        message: Option<&str>,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
    fn send_heartbeat(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
}

impl GameConnection {
    async fn send_player_info_to_opponent(
        &mut self,
        player: &PlayerConnection,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create a PlayerConnection to hold your information and send it to your opponent.
        if let Err(e) = player
            .get_channel()
            .send(GameMessage::PlayerConnection(PlayerConnection::new(
                self.player.clone(),
                self.opponent_sender.clone().unwrap(),
            )))
            .await
        {
            return self
                .cleanup(Some(&format!("Error sending player to opponent: {:?}", e)))
                .await;
        }

        Ok(())
    }

    async fn add_player_to_queue(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.tx
            .send(GameServerRequest::AddPlayerToQueue {
                player_connection: PlayerConnection::new(
                    self.player.clone(),
                    self.opponent_sender.clone().unwrap(),
                ),
            })
            .await?;
        Ok(())
    }

    async fn wait_for_opponent(
        &mut self,
    ) -> Result<(PlayerConnection, bool), Box<dyn std::error::Error>> {
        let mut interval = interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.send_heartbeat().await?;
                }
                opponent = self.opponent_receiver.recv() => {
                    match opponent {
                        Some(GameMessage::PlayerConnection(pc)) => {
                            return Ok((pc, false));
                        }
                        Some(GameMessage::GameState(_)) => {
                            return Err("Received game state while expecting player connection".into());
                        }
                        Some(GameMessage::Request(_)) => {
                            return Err("Received request while expecting player connection".into());
                        }
                        None => {
                            return Err("Channel closed before receiving opponent".into());
                        }
                    }
                }
            }
        }
    }
}

impl GameConnectionTrait for GameConnection {
    fn new(connection: TcpStream, tx: mpsc::Sender<GameServerRequest>) -> Self {
        let (opponent_tx, opponent_rx) = mpsc::channel::<GameMessage>(5);
        let player_id = Player::new();
        GameConnection {
            connection,
            tx,
            player: player_id.clone(),
            game_state: GameState::new(Some(player_id), None, false),
            opponent_sender: Some(Arc::new(opponent_tx)),
            opponent_receiver: opponent_rx,
        }
    }

    async fn handshake(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 16];
        for i in 0..2 {
            let n = self.connection.read(&mut buffer).await?;
            if n == 0 {
                return self.cleanup(Some("Handshake: Connection closed")).await;
            }

            // Client should first send hello (or ok) message
            // The server will assign a player number to the client.
            // The user should then send another ok message
            // If the player instead responds with a player id, the server will assign the player number to the client.
            match n {
                4 => {
                    let request =
                        Request(u32::from_be_bytes(buffer[..4].try_into().unwrap_or_else(
                            |_| panic!("Failed to convert buffer to u32 {:?}", &buffer[..4]),
                        )));
                    if i == 0 && request.is_ok_response() {
                        let bytes_written = self
                            .connection
                            .write(&self.player.get_id().into_bytes())
                            .await?;
                        if bytes_written != 16 {
                            return self.cleanup(Some("Failed to write player id")).await;
                        }
                    }
                }
                16 => {
                    if i == 0 {
                        return self.cleanup(Some("Invalid handshake message")).await;
                    }
                    self.player = Player::from_bytes(&buffer);
                    let bytes_written = self
                        .connection
                        .write(&Request::new_data_request(true).0.to_be_bytes())
                        .await?;
                    if bytes_written != 4 {
                        return self.cleanup(Some("Failed to write data request")).await;
                    }
                }
                _ => {
                    return self.cleanup(Some("Invalid handshake message")).await;
                }
            }
        }
        Ok(())
    }

    async fn get_opponent_and_initialize_state(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Request a user from the game state
        let (response_tx, response_rx) = oneshot::channel::<Option<PlayerConnection>>();
        self.tx
            .send(GameServerRequest::GetPlayerFromQueue {
                response: response_tx,
            })
            .await?;

        // Wait for game state to respond with an opponent
        let opponent = match response_rx.await {
            // Game server returned a player
            Ok(Some(player)) => {
                self.send_player_info_to_opponent(&player).await?;
                (player, true)
            }
            // Game server returned none which means the queue is empty
            Ok(None) => {
                self.add_player_to_queue().await?;
                self.wait_for_opponent().await?
            }
            Err(_) => return self.cleanup(Some("Error getting opponent")).await,
        };

        /* ### IMPORTANT ### */
        // Drop your own copy of the sender to keep only one reference to the sender Arc.
        // At this point the opponent has received the sender and is now responsible dropping it.
        self.opponent_sender = None;

        let opponent_player = opponent.0;
        let is_player_two = opponent.1;
        let bytes_written = self
            .connection
            .write(&opponent_player.get_player().get_id().into_bytes())
            .await?;
        if bytes_written != 16 {
            return self.cleanup(Some("Failed to write opponent id")).await;
        }

        self.opponent_sender = Some(opponent_player.get_channel().clone());
        self.game_state = GameState::new(
            Some(self.player.clone()),
            Some(opponent_player),
            is_player_two,
        );

        let bytes_written = self
            .connection
            .write(&self.game_state.to_request(false).0.to_be_bytes())
            .await?;
        if bytes_written != 4 {
            return self.cleanup(Some("Failed to write data request")).await;
        }

        Ok(())
    }

    async fn handle_request(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut delay = interval(Duration::from_secs(10));
        let mut buffer = [0u8; 4];

        loop {
            tokio::select! {
                _ = delay.tick() => {
                    // self.send_heartbeat().await?;
                }
                request = self.opponent_receiver.recv() =>
                {
                    match request {
                        Some(GameMessage::GameState(request)) => {
                            let bytes_written = self.connection.write(&request.to_request(false).0.to_be_bytes()).await?;
                            if bytes_written != 4 {
                                return self.cleanup(Some("Failed to write data request")).await;
                            }
                        }
                        Some(_) => {
                            return self.cleanup(Some("Invalid message from opponent")).await;
                        }
                        None => {
                            return self.cleanup(Some("Opponent has left the match.")).await;
                        }
                    }
                }
                request = self.connection.read(&mut buffer) => {
                    match request {
                        Ok(0) => self.cleanup(Some("Connection is closed")).await?,
                        Ok(4) => {
                            let request = Request(u32::from_be_bytes(buffer));
                            let new_state = GameState::from_request(request, self.player.clone())?;
                            if !self.game_state.validate_turn(&new_state)? {
                                return self.cleanup(Some("User sent an invalid request")).await;
                            }
                            self.opponent_sender.as_ref().unwrap().send(GameMessage::GameState(GameState::from_request(request, self.player.clone())?)).await?;
                            self.game_state = new_state;
                        },
                        Ok(_) => self.cleanup(Some("Invalid request")).await?,
                        Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) => self.cleanup(Some(&e.to_string())).await?,
                    }
                }
            }
        }
    }

    async fn send_heartbeat(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 4];

        let bytes_written = self
            .connection
            .write(&Request::new_data_request(true).0.to_be_bytes())
            .await?;
        if bytes_written != 4 {
            return self.cleanup(Some("Failed to write data request")).await;
        }

        match tokio::time::timeout(Duration::from_secs(5), self.connection.read(&mut buffer)).await
        {
            Ok(Ok(0)) => return self.cleanup(Some("Heartbeat: Connection closed")).await,
            Ok(Ok(4)) => {
                if !Request(u32::from_be_bytes(buffer)).is_ok_response() {
                    return self.cleanup(Some("Invalid heartbeat response")).await;
                }
            }
            Ok(Ok(_)) => return self.cleanup(Some("Invalid heartbeat response")).await,
            Ok(Err(e)) => return self.cleanup(Some(&e.to_string())).await,
            Err(_) => return self.cleanup(Some("Heartbeat: Timeout")).await,
        }

        Ok(())
    }

    async fn cleanup(&mut self, message: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let (response_tx, response_rx) = oneshot::channel::<()>();
        self.tx
            .send(GameServerRequest::RemovePlayerFromQueue {
                player: self.player.clone(),
                response: response_tx,
            })
            .await
            .unwrap_or_else(|e| println!("Error removing player from queue: {:?}", e));

        match response_rx.await {
            Ok(_) => println!("Player removed from queue"),
            Err(e) => println!("Error removing player from queue: {:?}", e),
        }

        self.connection
            .shutdown()
            .await
            .unwrap_or_else(|e| println!("Error shutting down connection: {:?}", e));

        match message {
            Some(msg) => Err(msg.into()),
            None => Ok(()),
        }
    }
}
