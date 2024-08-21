use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Player(Uuid);

pub trait PlayerTrait {
    fn new() -> Self;
    fn get_id(&self) -> &Uuid;
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
        assert_eq!(player.get_id().to_string(), "0b0e0e0f-0b0e-0e0f-0b0e-0e0f0b0e0e0f");
    }
}