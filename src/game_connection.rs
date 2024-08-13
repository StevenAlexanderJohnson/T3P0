use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};

use crate::{request::Request, DataRequest, GameRequest, Player, PlayerTrait};

pub struct GameConnection {
    player: Player,
    connection: TcpStream,
    tx: mpsc::Sender<GameRequest>,
}

trait GameConnectionTrait {
    fn new(player: Player, connection: TcpStream, tx: mpsc::Sender<GameRequest>) -> Self;
    async fn handshake(&mut self) -> Result<(), Box<dyn std::error::Error>>;
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
}
