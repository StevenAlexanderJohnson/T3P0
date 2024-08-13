use t3p0::{
    game_server, request::Request, DataRequest, GameRequest, GameServerTrait, GameState,
    GameStateTrait, Player, PlayerTrait,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    let (tx, mut rx) = mpsc::channel::<GameRequest>(32);
    let game_server = game_server::GameServer::new();

    // Create a thread that will manage the server state
    tokio::spawn(async move {
        while let Some(request) = rx.recv().await {
            game_server.handle_request(request).await;
        }
    });

    loop {
        let (socket, _) = listener.accept().await?;
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, tx_clone).await {
                eprintln!("Error: {:?}", e);
            }
        });
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    tx: mpsc::Sender<GameRequest>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 4];
    let mut player = Player::new();
    println!("New connection: {}", socket.peer_addr()?);
    println!("Player: {:?}", player);
    // Handshake
    for i in 0..2 {
        let n = socket.read(&mut buffer).await?;
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
                    socket.write(&player.get_id().to_bytes_le()).await?;
                }
            }
            16 => {
                if i == 0 {
                    return Err("Invalid handshake message".into());
                }
                let mut uuid_buffer = [0u8; 16];
                uuid_buffer[..4].copy_from_slice(&buffer);
                socket.read_exact(&mut uuid_buffer[4..]).await?;
                player = Player::from_bytes(&uuid_buffer);
                socket
                    .write(&Request::new_data_request(true).0.to_be_bytes())
                    .await?;
            }
            _ => {
                return Err("Invalid handshake message".into());
            }
        }
    }

    // Event loop
    loop {
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        if n != 4 {
            return Err("Invalid request".into());
        }

        let request = Request(u32::from_be_bytes(buffer));
        // If the request is not a valid request, we break the loop
        // If it is an ok request send an ok request back.
        // If the user doesn't receive the ok request, they will close the connection and try again.

        let (response_tx, response_rx) = oneshot::channel::<Option<GameState>>();
        tx.send(GameRequest::GetState {
            player_id: player.clone(),
            response: response_tx,
        })
        .await?;

        match response_rx.await {
            Ok(Some(game_state)) => {
                let _ = socket.write(&game_state.to_request().0.to_be_bytes());
            }
            Ok(None) => {
                let _ = socket.write(&Request::new_data_request(false).0.to_be_bytes());
            }
            Err(_) => {
                socket.write(&request.0.to_be_bytes()).await?;
            }
        }
    }
    Ok(())
}
