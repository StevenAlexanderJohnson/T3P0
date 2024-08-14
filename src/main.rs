use std::{collections::HashMap, fmt::Debug, sync::Arc};
use t3p0::{request::Request, DataRequest, GameState, GameStateTrait, Player, PlayerTrait};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot, Mutex},
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

#[derive(Debug)]
enum PlayerRequest {
    GetPlayer {
        response: oneshot::Sender<Option<(Player, oneshot::Sender<Player>)>>,
    },
    AddPlayer {
        player: Player,
        channel: oneshot::Sender<Player>,
    },
    TryRemovePlayer {
        player: Player,
    },
    SendMessage {
        player: Player,
        message: [u8; 16],
        response: oneshot::Sender<Option<String>>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8000").await?;

    let (game_state_tx, mut game_state_rx) = mpsc::channel::<GameRequest>(32);
    let game_state_map = Arc::new(Mutex::new(HashMap::<Player, GameState>::new()));

    let (player_queue_tx, mut player_queue_rx) = mpsc::channel::<PlayerRequest>(32);
    let game_queue = Arc::new(Mutex::new(Vec::<(Player, oneshot::Sender<Player>)>::new()));
    let player_map = Arc::new(Mutex::new(HashMap::<Player, TcpStream>::new()));

    let game_state_map_clone = game_state_map.clone();
    tokio::spawn(async move {
        while let Some(request) = game_state_rx.recv().await {
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

    let game_queue_clone = game_queue.clone();
    let player_map_clone = player_map.clone();
    tokio::spawn(async move {
        while let Some(player_request) = player_queue_rx.recv().await {
            match player_request {
                PlayerRequest::GetPlayer { response } => {
                    let mut queue = game_queue_clone.lock().await;
                    if queue.len() > 0 {
                        let _ = response.send(Some(queue.remove(0)));
                    } else {
                        let _ = response.send(None);
                    }
                }
                PlayerRequest::AddPlayer { player, channel } => {
                    let mut queue = game_queue_clone.lock().await;
                    println!("Adding {:?} to queue", player);
                    queue.push((player, channel))
                }
                PlayerRequest::TryRemovePlayer { player } => {
                    let mut queue = game_queue_clone.lock().await;
                    if let Some(index) = queue.iter().position(|p| p.0 == player) {
                        queue.remove(index);
                    } else {
                        panic!("Failed to remove player {:?}", player);
                    }
                }
                PlayerRequest::SendMessage {
                    player,
                    message,
                    response,
                } => {
                    let mut player_map = player_map_clone.lock().await;
                    if let Some(player) = player_map.get_mut(&player) {
                        let _ = match player.write(&message).await {
                            Ok(_) => response.send(None),
                            Err(e) => response.send(Some(e.to_string())),
                        };
                    } else {
                        let _ =
                            response.send(Some(String::from("The player request does not exist.")));
                    }
                }
            }
        }
    });

    loop {
        let (socket, _) = listener.accept().await?;
        let tx_clone = game_state_tx.clone();
        let player_tx_clone = player_queue_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, tx_clone, player_tx_clone).await {
                eprintln!("Error: {:?}", e);
            }
        });
    }
}

async fn handle_connection(
    mut socket: TcpStream,
    game_state_tx: mpsc::Sender<GameRequest>,
    player_queue_tx: mpsc::Sender<PlayerRequest>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 4];
    let mut player = Player::new();
    let mut game_state = GameState::new(Some(player.clone()), None);
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
                if request.is_ok_response() {
                    if i == 0 {
                        socket.write(&player.get_id().into_bytes()).await?;
                    }
                } else {
                    return Err("Invalid handshake Ok".into());
                }
            }
            16 => {
                if i == 0 {
                    return Err("Invalid handshake message".into());
                }
                let mut uuid_buffer = [0u8; 16];
                uuid_buffer[..4].copy_from_slice(&buffer);
                socket.read_exact(&mut uuid_buffer[4..]).await?;
                println!("Player requested ID: {:?}", uuid_buffer);
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

    // Find an opponent or add yourself to queue.
    let (response_tx, response_rx) =
        oneshot::channel::<Option<(Player, oneshot::Sender<Player>)>>();
    player_queue_tx
        .send(PlayerRequest::GetPlayer {
            response: response_tx,
        })
        .await?;
    match response_rx.await {
        Ok(player_rx) => {
            if let Some(p) = player_rx {
                println!("Found {:?} to face {:?}", p, player);
            } else {
                let (ready_tx, ready_rv) = oneshot::channel::<Player>();
                player_queue_tx
                    .send(PlayerRequest::AddPlayer {
                        player: player.clone(),
                        channel: ready_tx,
                    })
                    .await?;
                match ready_rv.await {
                    Ok(p2) => game_state.set_players(Some(Box::new([player.clone(), p2]))),
                    Err(e) => {
                        game_state.set_players(None);
                        println!("Error getting player from ready channel: {:?}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("Error while receiving player to face: {:?}", e);
            return Err("An error occurred while receiving a player to face.".into());
        }
    }

    // Event loop
    loop {
        println!("Entering loop");

        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            println!("{:?} ended their connection", player);
            player_queue_tx
                .send(PlayerRequest::TryRemovePlayer { player })
                .await?;
            break;
        }
        if n != 4 {
            return Err("Invalid request".into());
        }

        let request = Request(u32::from_be_bytes(buffer));
        println!("{:?}", request);
        // If the request is not a valid request, we break the loop
        // If it is an ok request send an ok request back.
        // If the user doesn't receive the ok request, they will close the connection and try again.

        let (response_tx, mut response_rx) = mpsc::channel::<Option<GameState>>(1);
        game_state_tx
            .send(GameRequest::GetState {
                player_id: player.clone(),
                response: response_tx,
            })
            .await?;

        if let Some(game_state_rec) = response_rx.recv().await {
            if let Some(game_state) = game_state_rec {
                socket
                    .write(&game_state.to_request().0.to_be_bytes())
                    .await?;
            } else {
                socket.write(&request.0.to_be_bytes()).await?;
            }
        }
    }
    Ok(())
}
