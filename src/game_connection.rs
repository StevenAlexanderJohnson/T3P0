use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, oneshot},
};

use crate::{
    request::Request, DataRequest, GameRequest, GameState, GameStateTrait, Player, PlayerTrait,
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
}

impl GameConnectionTrait for GameConnection {
    fn new(connection: TcpStream, tx: mpsc::Sender<GameRequest>) -> Self {
        GameConnection {
            connection,
            tx,
            player: Player::new(),
            game_state: None,
        }
    }

    async fn handshake(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 16];
        for i in 0..2 {
            let n = self.connection.read(&mut buffer).await?;
            if n == 0 {
                return Err("Connection closed".into());
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
                            return Err("Failed to write player id".into());
                        }
                    }
                }
                16 => {
                    if i == 0 {
                        return Err("Invalid handshake message".into());
                    }
                    self.player = Player::from_bytes(&buffer);
                    let bytes_written = self
                        .connection
                        .write(&Request::new_data_request(true).0.to_be_bytes())
                        .await?;
                    if bytes_written != 4 {
                        return Err("Failed to write data request".into());
                    }
                }
                _ => {
                    return Err("Invalid handshake message".into());
                }
            }
        }
        Ok(())
    }

    async fn handle_request(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let mut buffer = [0u8; 4];
            let n = self.connection.read(&mut buffer).await?;
            if n == 0 {
                return Err("Connection closed".into());
            }
            if n != 4 {
                return Err("Invalid request".into());
            }

            let request = Request(u32::from_be_bytes(buffer));
            // If the request is not a valid request, we break the loop
            // If it is an ok request send an ok request back.
            // If the user doesn't receive the ok request, they will close the connection and try again.

            let (response_tx, response_rx) = oneshot::channel::<Option<GameState>>();
            self.tx
                .send(GameRequest::GetState {
                    player_id: self.player.clone(),
                    response: response_tx,
                })
                .await?;

            match response_rx.await {
                Ok(Some(game_state)) => {
                    let _ = self
                        .connection
                        .write(&game_state.to_request().0.to_be_bytes())
                        .await?;
                }
                Ok(None) => {
                    let bytes_written = self
                        .connection
                        .write(&Request::new_data_request(false).0.to_be_bytes())
                        .await?;
                    if bytes_written != 4 {
                        return Err("Failed to write data request".into());
                    }
                }
                Err(_) => {
                    let bytes_written = self.connection.write(&request.0.to_be_bytes()).await?;
                    if bytes_written != 4 {
                        return Err("Failed to write request".into());
                    }
                }
            };
        }
    }

    async fn get_opponent_and_initialize_state(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (response_tx, response_rx) =
            oneshot::channel::<Option<(Player, oneshot::Sender<Player>)>>();
        self.tx
            .send(GameRequest::GetPlayerFromQueue {
                response: response_tx,
            })
            .await?;

        let opponent = match response_rx.await {
            Ok(Some(player)) => {
                // Send self to the person waiting for an opponent.
                match player.1.send(self.player.clone()) {
                    Ok(_) => {}
                    Err(_) => return Err("Error sending player to opponent".into()),
                }
                player.0
            }
            Ok(None) => {
                println!("No opponent found. Adding self to queue.");
                let (response_tx, response_rx) = oneshot::channel::<Player>();

                // Add self to the queue
                self.tx
                    .send(GameRequest::AddPlayerToQueue {
                        player: self.player.clone(),
                        response: response_tx,
                    })
                    .await?;

                // Wait for an opponent
                match response_rx.await {
                    Ok(player) => player,
                    Err(_) => return Err("Error adding player to queue".into()),
                }
            }
            Err(_) => return Err("Error getting opponent".into()),
        };
        println!("INITIALIZING GAME STATE");
        println!("ME: {:?}\nOPPONENT: {:?}", self.player, opponent);
        self.game_state = Some(GameState::new(
            Some(self.player.clone()),
            Some([self.player.clone(), opponent.clone()]),
        ));

        let bytes_written = self
            .connection
            .write(&opponent.get_id().into_bytes())
            .await?;
        if bytes_written != 16 {
            return Err("Failed to write opponent id".into());
        }

        Ok(())
    }
}
