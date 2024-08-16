use t3p0::{
    game_connection::{self, GameConnectionTrait},
    game_server, GameRequest, GameServerTrait,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    println!("Server listening on port 8000");
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
    socket: TcpStream,
    tx: mpsc::Sender<GameRequest>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("New Connection");
    let mut connection = game_connection::GameConnection::new(socket, tx);
    connection.handshake().await?;
    connection.get_opponent_and_initialize_state().await?;
    // Event loop
    // connection.handle_request().await
    Ok(())
}
