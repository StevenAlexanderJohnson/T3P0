use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, oneshot, Mutex},
    time::{interval, Duration, Instant},
};

use crate::{
    player::{PlayerConnection, PlayerConnectionTrait},
    request::Request,
    DataRequest, GameRequest, GameState, GameStateTrait, Player, PlayerTrait,
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
    game_state: Option<GameState>,
    tx: mpsc::Sender<GameRequest>,
    opponent_sender: Arc<Mutex<mpsc::Sender<GameState>>>,
    opponent_receiver: Arc<Mutex<mpsc::Receiver<GameState>>>,
}

pub trait GameConnectionTrait {
    /// Create a new GameConnection
    ///
    /// # Arguments
    ///
    /// * `player` - The player that is connected to the server.
    /// * `connection` - The connection to the server.
    /// * `tx` - The sending channel to send requests to the main thread.
    fn new(connection: TcpStream, tx: mpsc::Sender<GameRequest>) -> Self;
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

impl GameConnectionTrait for GameConnection {
    fn new(connection: TcpStream, tx: mpsc::Sender<GameRequest>) -> Self {
        let (opponent_tx, opponent_rx) = mpsc::channel::<GameState>(5);
        GameConnection {
            connection,
            tx,
            player: Player::new(),
            game_state: None,
            opponent_sender: Arc::new(Mutex::new(opponent_tx)),
            opponent_receiver: Arc::new(Mutex::new(opponent_rx)),
        }
    }

    async fn handshake(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 16];
        for i in 0..2 {
            let n = self.connection.read(&mut buffer).await?;
            if n == 0 {
                return self.cleanup(Some("Connection closed")).await;
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

    async fn handle_request(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let opponent_message = self.opponent_receiver.lock().await.try_recv();
            match opponent_message {
                Ok(request) => {
                    println!("{:?}", request)
                }
                Err(mpsc::error::TryRecvError::Empty) => (),
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.cleanup(Some("Opponent has left the match.")).await?
                }
            };

            let mut buffer = [0u8; 4];
            match self.connection.try_read(&mut buffer) {
                Ok(0) => self.cleanup(Some("Connection is closed")).await?,
                Ok(n) => println!("read {} bytes", n),
                Err(ref e) if e.kind() == tokio::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => self.cleanup(Some(&e.to_string())).await?,
            };
        }
    }

    async fn get_opponent_and_initialize_state(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Request a user from the game state
        let (response_tx, response_rx) = oneshot::channel::<Option<PlayerConnection>>();
        self.tx
            .send(GameRequest::GetPlayerFromQueue {
                response: response_tx,
            })
            .await?;

        // Wait for game state to respond with an opponent
        let opponent = match response_rx.await {
            // Game server returned a player
            Ok(Some(player)) => {
                let pc = PlayerConnection::new(self.player.clone(), player.get_channel().clone());
                // Send self to the person waiting for an opponent.
                let mut game_state =
                    GameState::from_request(Request::new_data_request(true), self.player.clone())?;
                game_state.set_opponent(Some(pc));

                if let Err(e) = player.get_channel().lock().await.send(game_state).await {
                    return self
                        .cleanup(Some(&format!("Error sending player to opponent: {:?}", e)))
                        .await;
                }
                player
            }
            // Game server returned none which means the queue is empty
            Ok(None) => {
                println!("No opponent found. Adding self to queue.");
                self.tx
                    .send(GameRequest::AddPlayerToQueue {
                        player_connection: PlayerConnection::new(
                            self.player.clone(),
                            self.opponent_sender.clone(),
                        ),
                    })
                    .await?;

                let mut interval = interval(Duration::from_secs(1));
                let timeout = Duration::from_secs(5);
                let start = Instant::now();

                loop {
                    interval.tick().await;
                    // Wait for a response from game state
                    let response = self.opponent_receiver.lock().await.try_recv();

                    match response {
                        Ok(response) => {
                            if !response.to_request().is_ok_response()
                                || response.get_opponent().is_none()
                            {
                                return self.cleanup(Some("Invalid response from game state")).await;
                            }
                            break response.get_opponent().unwrap();
                        }
                        Err(mpsc::error::TryRecvError::Empty) => {
                            self.send_heartbeat().await?;
                            if start.elapsed() >= timeout {
                                return self.cleanup(Some("Timeout waiting for opponent")).await;
                            }
                        }
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            println!("Disconnected");
                            return self.cleanup(Some("Channel closed before receiving opponent")).await;
                        }
                    }
                }
            }
            Err(_) => return self.cleanup(Some("Error getting opponent")).await,
        };
        self.game_state = Some(GameState::new(
            Some(self.player.clone()),
            Some(PlayerConnection::new(
                opponent.get_player().clone(),
                opponent.get_channel().clone(),
            )),
        ));

        let bytes_written = self
            .connection
            .write(&opponent.get_player().get_id().into_bytes())
            .await?;
        if bytes_written != 16 {
            return self.cleanup(Some("Failed to write opponent id")).await;
        }

        Ok(())
    }

    async fn cleanup(&mut self, message: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let (response_tx, response_rx) = oneshot::channel::<()>();
        self.tx
            .send(GameRequest::RemovePlayerFromQueue {
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

    async fn send_heartbeat(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 4];

        let bytes_written = self
            .connection
            .write(&Request::new_data_request(true).0.to_be_bytes())
            .await?;
        if bytes_written != 4 {
            return self.cleanup(Some("Failed to write data request")).await;
        }

        let n = self.connection.read(&mut buffer).await?;
        if n == 0 {
            return self.cleanup(Some("Connection closed")).await;
        }

        if !Request(u32::from_be_bytes(buffer)).is_ok_response() {
            return self.cleanup(Some("Invalid heartbeat response")).await;
        }

        Ok(())
    }
}
