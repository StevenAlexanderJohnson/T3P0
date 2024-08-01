use std::{collections::HashMap, sync::Arc};
use t3p0::{game_state::PlayerNumber, request::Request, DataRequest, GameState, GameStateTrait};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, Mutex},
};

#[derive(Debug)]
enum GameRequest {
    GetState {
        game_id: u32,
        response: mpsc::Sender<Option<GameState>>,
    },
    UpdateState {
        game_id: u32,
        new_state: GameState,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    let (tx, mut rx) = mpsc::channel::<GameRequest>(32);
    let game_state_map = Arc::new(Mutex::new(HashMap::<u32, GameState>::new()));

    let game_state_map_clone = game_state_map.clone();
    tokio::spawn(async move {
        while let Some(request) = rx.recv().await {
            let mut state = game_state_map_clone.lock().await;
            match request {
                GameRequest::GetState { game_id, response } => {
                    let game_state = state.get(&game_id).cloned();
                    let _ = response.send(game_state);
                }
                GameRequest::UpdateState { game_id, new_state } => {
                    state.insert(game_id, new_state);
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

    loop {
        // Initialize handshare
        // User sends ok response
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        let request: Request = Request(u32::from_be_bytes(buffer));
        // If the request is not a valid request, we break the loop
        // If it is an ok request send an ok request back.
        // If the user doesn't receive the ok request, they will close the connection and try again.
        if request.is_ok_response() {
            socket.write(&request.0.to_be_bytes()).await?;
        } else {
            break;
        }

        let (response_tx, mut response_rx) = mpsc::channel::<Option<GameState>>(1);
        tx.send(GameRequest::GetState {
            game_id: request.0,
            response: response_tx,
        })
        .await?;

        if let Some(game_state) = response_rx.recv().await {
            if game_state.is_none() {
                let game_state = GameState::new(Some(PlayerNumber(1)), [PlayerNumber(0); 2]);
                socket.write(&game_state.to_request().0.to_be_bytes()).await?;
                tx.send(GameRequest::UpdateState {
                    game_id: 1u32,
                    new_state: game_state,
                }).await?;
            }
            tx.send(GameRequest::UpdateState {
                game_id: 1u32,
                new_state: GameState::from_request(request, PlayerNumber(2))?,
            })
            .await?;

            socket
                .write(&Request::new_data_request(true).0.to_be_bytes())
                .await?;
        }
    }
    Ok(())
}
