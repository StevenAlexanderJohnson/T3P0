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
    fn new(player: Player, connection: TcpStream, tx: mpsc::Sender<GameRequest>) -> Self;
    /// Handles the handshake between the client and the server.
    fn handshake(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
    /// Handles the request from the client.
    fn handle_request(
        &mut self,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
}

impl GameConnectionTrait for GameConnection {
    fn new(player: Player, connection: TcpStream, tx: mpsc::Sender<GameRequest>) -> Self {
        GameConnection {
            player,
            connection,
            tx,
        }
    }

    async fn handshake(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0u8; 4];
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
                    let request = Request(u32::from_be_bytes(buffer));
                    if i == 0 && request.is_ok_response() {
                        self.connection
                            .write(&self.player.get_id().to_bytes_le())
                            .await?;
                    }
                }
                16 => {
                    if i == 0 {
                        return Err("Invalid handshake message".into());
                    }
                    let mut uuid_buffer = [0u8; 16];
                    uuid_buffer[..4].copy_from_slice(&buffer);
                    self.connection.read_exact(&mut uuid_buffer[4..]).await?;
                    self.player = Player::from_bytes(&uuid_buffer);
                    self.connection
                        .write(&Request::new_data_request(true).0.to_be_bytes())
                        .await?;
                }
                _ => {
                    return Err("Invalid handshake message".into());
                }
            }
        }
        Ok(())
    }

    async fn handle_request(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
                    .write(&game_state.to_request().0.to_be_bytes());
            }
            Ok(None) => {
                let _ = self
                    .connection
                    .write(&Request::new_data_request(false).0.to_be_bytes());
            }
            Err(_) => {
                self.connection.write(&request.0.to_be_bytes()).await?;
            }
        };

        Ok(())
    }
}
