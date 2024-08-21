use std::sync::Arc;

use tokio::sync::mpsc;

use crate::{message::GameMessage, player::Player};

/// PlayerConnection is a struct that represents a connection to a player.
/// 
/// # Fields
/// 
/// * player - The player.
/// * channel - The channel to send messages to the player.
/// 
/// # Notes
/// 
/// PlayerConnection is not generally used to store data.
/// It's primary use is to send data across threads.
/// It should only be used once per connection.
#[derive(Debug, Clone)]
pub struct PlayerConnection {
    player: Arc<Player>,
    channel: Arc<mpsc::Sender<GameMessage>>,
}

pub trait PlayerConnectionTrait {
    /// Create a new PlayerConnection.
    /// 
    /// # Arguments
    /// 
    /// * player - The player.
    /// * channel - The channel to send messages to the player.
    /// 
    /// # Notes
    /// 
    /// Arguments are wrapped in Arcs to allow for sharing across threads.
    /// 
    /// # Returns
    /// 
    /// A new PlayerConnection.
    fn new(player: Arc<Player>, channel: Arc<mpsc::Sender<GameMessage>>) -> Self;

    /// Returns the player stored within the connection information.
    /// 
    /// # Returns
    /// 
    /// A reference to the player.
    fn get_player(&self) -> &Arc<Player>;

    /// Returns the channel that the connection uses to send messages across threads.
    /// 
    /// # Returns
    /// 
    /// A reference to the channel.
    fn get_channel(&self) -> &Arc<mpsc::Sender<GameMessage>>;
}

impl PlayerConnectionTrait for PlayerConnection {
    fn new(player: Arc<Player>, channel: Arc<mpsc::Sender<GameMessage>>) -> Self {
        PlayerConnection { player, channel }
    }

    fn get_player(&self) -> &Arc<Player> {
        &self.player
    }

    fn get_channel(&self) -> &Arc<mpsc::Sender<GameMessage>> {
        &self.channel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        game_state::{GameState, GameStateTrait},
        player::{Player, PlayerTrait},
    };

    #[tokio::test]
    async fn test_new() {
        let player = Arc::new(Player::new());
        let (tx, mut rx) = mpsc::channel(1);
        let arc_tx = Arc::new(tx);
        let player_connection = PlayerConnection::new(player.clone(), arc_tx.clone());
        assert_eq!(player_connection.player, player);

        let gs = GameState::new(true);
        let message = GameMessage::GameState(gs.clone());

        if let Err(_) = arc_tx.send(message).await {
            panic!("Failed to send message");
        }

        let output = match rx.recv().await {
            Some(GameMessage::GameState(game_state)) => game_state,
            _ => panic!("Expected GameState"),
        };

        assert_eq!(output.to_request(), gs.to_request())
    }
}
