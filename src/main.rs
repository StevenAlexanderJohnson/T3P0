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
        response: mpsc::Sender<GameState>,
    },
    UpdateState {
        game_id: u32,
        new_state: GameState,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    let (tx, mut rx) = mpsc::channel(32);
    let game_state_map = Arc::new(Mutex::new(HashMap::<u32, GameState>::new()));

    let game_state_map_clone = game_state_map.clone();
    tokio::spawn(async move {
        while let Some(request) = rx.recv().await {
            let mut state = game_state_map_clone.lock().await;
            match request {
                GameRequest::GetState { game_id, response } => {
                    let game_state = state.get(&game_id).cloned();
                    if let Some(game_state) = game_state {
                        let _ = response.send(game_state);
                    }
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
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            break;
        }

        let request: Request = Request(u32::from_be_bytes(buffer));
        let (response_tx, mut response_rx) = mpsc::channel::<GameState>(1);
        tx.send(GameRequest::GetState {
            game_id: request.0,
            response: response_tx,
        })
        .await?;

        if let Some(game_state) = response_rx.recv().await {
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
