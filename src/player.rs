use uuid::Uuid;

/// Player is a struct that represents a player.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Player(Uuid);

pub trait PlayerTrait {
    /// Create a new Player.
    /// 
    /// # Returns
    /// 
    /// A new Player with a random UUID.
    fn new() -> Self;

    /// Get the ID of the player.
    /// 
    /// # Returns
    /// 
    /// A reference to the player's ID.
    fn get_id(&self) -> &Uuid;

    /// Create a new Player from a byte array.
    /// 
    /// # Arguments
    /// 
    /// * bytes - A byte array of length 16.
    /// 
    /// # Notes
    /// 
    /// This is used when connections are negotiating player IDs, it should be used at most once per connection.
    /// 
    /// # Returns
    /// 
    /// A new Player.
    fn from_bytes(bytes: &[u8; 16]) -> Self;
}

impl PlayerTrait for Player {
    fn new() -> Self {
        Player(Uuid::new_v4())
    }

    fn get_id(&self) -> &Uuid {
        &self.0
    }

    fn from_bytes(bytes: &[u8; 16]) -> Self {
        Player(*Uuid::from_bytes_ref(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let player = Player::new();
        assert_eq!(player.get_id().to_string().len(), 36);
    }

    #[test]
    fn test_from_bytes() {
        let bytes = [
            0x0b, 0x0e, 0x0e, 0x0f, 0x0b, 0x0e, 0x0e, 0x0f, 0x0b, 0x0e, 0x0e, 0x0f, 0x0b, 0x0e,
            0x0e, 0x0f,
        ];
        let player = Player::from_bytes(&bytes);
        assert_eq!(player.0.to_string(), "0b0e0e0f-0b0e-0e0f-0b0e-0e0f0b0e0e0f");
    }

    #[test]
    fn test_get_id() {
        let bytes = [
            0x0b, 0x0e, 0x0e, 0x0f, 0x0b, 0x0e, 0x0e, 0x0f, 0x0b, 0x0e, 0x0e, 0x0f, 0x0b, 0x0e,
            0x0e, 0x0f,
        ];
        let player = Player::from_bytes(&bytes);
        assert_eq!(
            player.get_id().to_string(),
            "0b0e0e0f-0b0e-0e0f-0b0e-0e0f0b0e0e0f"
        );
    }
}
