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
    opponent: Option<Player>,
    connection: TcpStream,
    game_state: GameState,
    tx: mpsc::Sender<GameServerRequest>,
    opponent_sender: Arc<mpsc::Sender<GameMessage>>,
    opponent_receiver: mpsc::Receiver<GameMessage>,
    is_p2: bool,
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

    /// Performs the handshake with the client to establish a connection.
    ///
    /// The handshake is a two step process:
    /// 1. The client sends a hello message to the server.
    /// 2. The server responds with a player ID.
    /// 3. The client responds with one of two responses:
    ///    - If the client responds with an ok message, the server assigns the ID from step 2 to the client.
    ///    - If the client responds with a player id, the server will assign the player number to the client and respond with OK.
    ///
    /// # Returns
    ///
    /// A Result with an empty Ok or an error message.
    ///
    /// # Errors
    ///
    /// If the client sends an invalid message or the connection is closed.
    fn handshake(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    /// Gets an opponent from the game server.
    ///
    /// If there is an opponent in the queue, the opponent is sent to the client.
    /// If there is no opponent in the queue, the client is added to the queue and the client waits for an opponent.
    ///
    /// After getting a response from the game server, the client sends the opponent ID to the client.
    ///
    /// # Returns
    ///
    /// A Result with an empty Ok or an error message.
    fn get_opponent(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    /// Initializes the game state for the client.
    ///
    /// The game state is sent to the client.
    ///
    /// # Returns
    ///
    /// A Result with an empty Ok or an error message.
    fn initialize_state(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    /// Handles requests from the client.
    ///
    /// The client can send a request to the server to update the game state.
    /// The server will validate the request and update the game state.
    ///
    /// The server will also send the updated game state to the opponent.
    ///
    /// # Returns
    ///
    /// A Result with an empty Ok or an error message.
    ///
    /// # Errors
    ///
    /// - If the client sends an invalid request or the connection is closed.
    /// - If the opponent sends an invalid message or the connection is closed.
    /// - If the opponent leaves the match.
    /// - If the client sends an invalid heartbeat response.
    fn handle_request(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    /// Cleans up the connection.
    ///
    /// The client is removed from the queue and the connection is closed.
    ///
    /// # Arguments
    ///
    /// * `message` - An optional message to display when cleaning up.
    ///
    /// # Returns
    ///
    /// A Result with an empty Ok or an error message.
    ///
    /// # Errors
    ///
    /// If there is an error removing the player from the queue or shutting down the connection.
    ///
    /// # Notes
    ///
    /// This function is normally called inside the other functions to handle errors.
    ///
    /// # Example
    ///
    /// ```
    /// match self.get_opponent().await {
    ///     Ok(_) => println!("Opponent found"),
    ///     Err(e) => self.cleanup(Some(&e.to_string())).await,
    /// }
    fn cleanup(
        &mut self,
        message: Option<&str>,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;

    /// Sends a heartbeat to the client.
    ///
    /// The client should respond with an ok message after receiving the heartbeat or the connection will be closed.
    ///
    /// # Returns
    ///
    /// A Result with an empty Ok or an error message.
    ///
    /// # Errors
    ///
    /// - If the client does not respond with an ok message.
    /// - If the connection is closed.
    /// - The client responds with an invalid message.
    fn send_heartbeat(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
}

impl GameConnection {
    async fn send_message_to_opponent(&mut self, message: GameMessage) -> Result<(), Box<dyn std::error::Error>> {
        if let Err(e) = self.opponent_sender.send(message).await {
            return self.cleanup(Some(&format!("Error sending message to opponent: {:?}", e))).await;
        }

        Ok(())
    }
    async fn trade_player_info(
        &mut self,
        player: &PlayerConnection,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create a PlayerConnection to hold your information and send it to your opponent.
        if let Err(e) = player
            .get_channel()
            .send(GameMessage::PlayerConnection(PlayerConnection::new(
                self.player.clone(),
                self.opponent_sender.clone(),
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
                    self.opponent_sender.clone(),
                ),
            })
            .await?;
        Ok(())
    }

    async fn wait_for_opponent(
        &mut self,
    ) -> Result<PlayerConnection, Box<dyn std::error::Error>> {
        let mut interval = interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.send_heartbeat().await?;
                }
                opponent = self.opponent_receiver.recv() => {
                    match opponent {
                        Some(GameMessage::PlayerConnection(pc)) => {
                            return Ok(pc);
                        }
                        Some(GameMessage::GameState(_)) => {
                            return Err("Received game state while expecting player connection".into());
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
        GameConnection {
            connection,
            tx,
            player: Player::new(),
            opponent: None,
            game_state: GameState::new(false),
            opponent_sender: Arc::new(opponent_tx),
            opponent_receiver: opponent_rx,
            is_p2: false,
        }
    }

    async fn handshake(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 16];
        for i in 0..2 {
            // Client should first send hello (or ok) message
            // The server will assign a player number to the client.
            // The user should then send another ok message
            // If the player instead responds with a player id, the server will assign the player number to the client.
            match self.connection.read(&mut buffer).await? {
                0 => return self.cleanup(Some("Handshake: Connection closed")).await,
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

    async fn get_opponent(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
                self.trade_player_info(&player).await?;
                self.is_p2 = true;
                player
            }
            // Game server returned none which means the queue is empty
            Ok(None) => {
                self.add_player_to_queue().await?;
                self.is_p2 = false;
                self.wait_for_opponent().await?
            }
            Err(_) => return self.cleanup(Some("Error getting opponent")).await,
        };


        self.opponent = Some(opponent.get_player().clone());

        let bytes_written = self
            .connection
            .write(&opponent.get_player().get_id().into_bytes())
            .await?;
        if bytes_written != 16 {
            return self.cleanup(Some("Failed to write opponent id")).await;
        }

        /* ### IMPORTANT ### */
        // Drop your own copy of the sender to keep only one reference to the sender Arc.
        // At this point the opponent has received the sender and is now responsible dropping it.
        self.opponent_sender = opponent.get_channel().clone();
        Ok(())
    }

    async fn initialize_state(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.game_state = GameState::new(self.is_p2);

        if 4 != self
            .connection
            .write(&self.game_state.to_request().0.to_be_bytes())
            .await?
        {
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
                    self.send_heartbeat().await?;
                }
                request = self.opponent_receiver.recv() =>
                {
                    match request {
                        Some(GameMessage::GameState(game_state)) => {
                            self.game_state.update_board(&game_state, !self.is_p2);
                            let bytes_written = self.connection.write(&game_state.to_request().0.to_be_bytes()).await?;
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
                            let new_state = GameState::from_request(request)?;
                            if !self.game_state.validate_board(&new_state, self.is_p2) || !self.game_state.validate_turn(&new_state) {
                                return self.cleanup(Some("User sent an invalid request")).await;
                            }
                            self.game_state.update_board(&new_state, self.is_p2);
                            self.send_message_to_opponent(GameMessage::GameState(new_state)).await?;
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
            None => Err("Cleanup successful".into()),
        }
    }
}
