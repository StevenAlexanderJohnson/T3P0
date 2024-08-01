use std::{collections::HashMap, sync::Arc};
use t3p0::{
    request::Request, DataRequest, GameState, GameStateTrait, Player,
    PlayerTrait,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};

#[derive(Debug)]
enum GameRequest {
    GetState {
        player_id: Player,
        response: mpsc::Sender<Option<GameState>>,
    },
    UpdateState {
        player_id: Player,
        new_state: GameState,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    let (tx, mut rx) = mpsc::channel::<GameRequest>(32);
    let game_state_map = Arc::new(Mutex::new(HashMap::<Player, GameState>::new()));

    let game_state_map_clone = game_state_map.clone();
    tokio::spawn(async move {
        while let Some(request) = rx.recv().await {
            let mut state = game_state_map_clone.lock().await;
            match request {
                GameRequest::GetState {
                    player_id,
                    response,
                } => {
                    let game_state = state.get(&player_id).cloned();
                    let _ = response.send(game_state);
                }
                GameRequest::UpdateState {
                    player_id,
                    new_state,
                } => {
                    state.insert(player_id, new_state);
                }
            }
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

        let (response_tx, mut response_rx) = mpsc::channel::<Option<GameState>>(1);
        tx.send(GameRequest::GetState {
            player_id: player.clone(),
            response: response_tx,
        })
        .await?;

        if let Some(game_state) = response_rx.recv().await {
            if game_state.is_none() {
                let game_state = GameState::new(Some(player.clone()), [player.clone(), Player::new()]);
                socket
                    .write(&game_state.to_request().0.to_be_bytes())
                    .await?;
                tx.send(GameRequest::UpdateState {
                    player_id: player.clone(),
                    new_state: game_state,
                })
                .await?;
            }
            tx.send(GameRequest::UpdateState {
                player_id: player.clone(),
                new_state: GameState::from_request(request, Player::new())?,
            })
            .await?;

            socket
                .write(&Request::new_data_request(true).0.to_be_bytes())
                .await?;
        }
    }
    Ok(())
}
