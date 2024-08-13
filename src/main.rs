use t3p0::{
    game_connection::{self, GameConnectionTrait},
    game_server, GameRequest, GameServerTrait, Player, PlayerTrait,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
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
    socket: TcpStream,
    tx: mpsc::Sender<GameRequest>,
) -> Result<(), Box<dyn std::error::Error>> {
    let player = Player::new();
    let mut connection = game_connection::GameConnection::new(player, socket, tx);
    connection.handshake().await?;

    // Event loop
    loop {
        connection.handle_request().await?;
    }
}
