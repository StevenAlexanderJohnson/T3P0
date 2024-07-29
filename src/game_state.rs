use crate::request::DataRequest;

struct GameState {
    board: [u8; 9],
    turn: u8,
    message_number: u8,
    p2_turn: bool
}

trait GameStateTrait {
    fn new() -> Self;
    fn from_request(request: u32) -> Option<Self> where Self: Sized;
    fn p2_turn(&self) -> bool;
    fn compare_boards(&self, other: &GameState) -> bool;
}

impl GameStateTrait for GameState {
    fn new() -> Self {
        GameState {
            turn: 0,
            p2_turn: true,
            message_number: 0,
            board: [0u8; 9],
        }
    }

    /// Create a new GameState from a request
    /// 
    /// # Arguments
    /// 
    /// * `request` - A u32 that represents the request
    /// 
    /// # Returns
    /// 
    /// * `Option<Self>` - A new GameState if the request is valid, None otherwise
    fn from_request(request: u32) -> Option<Self> {
        let is_p2_turn = request.get_is_p2_turn();
        let turn = request.get_turn();
        let message_number = request.get_message_number();

        if message_number % 2 == 1 && !is_p2_turn {
            return None;
        }
        if message_number % 2 == 0 && is_p2_turn {
            return None;
        }
        if turn > message_number {
            return None;
        }

        let mut board = [0u8; 9];
        let board_state = request.get_board_state();
        for i in 0..9 {
            board[i] = (board_state >> i) as u8 & 1;
        }

        Some(GameState {
            board: board, 
            turn: request.get_turn(),
            message_number: request.get_message_number(),
            p2_turn: request.get_is_p2_turn(),
        })
    }

    fn p2_turn(&self) -> bool {
        self.p2_turn
    }
    
    fn compare_boards(&self, other: &GameState) -> bool {
        let mut differences = 0;
        for i in 0..9 {
            // If the board is changing a value that is already changed, it is not a valid move
            if self.board[i] != 0 && self.board[i] != other.board[i] {
                return false;
            }
            if self.board[i] != other.board[i] {
                differences += 1;
            }
        }
        differences == 1
    }
}


#[cfg(test)]
mod test {
    use crate::request::Bits;

    use super::*;

    #[test]
    fn test_new() {
        let gs = GameState::new();
        assert_eq!(gs.board, [0u8; 9]);
        assert_eq!(gs.turn, 0);
        assert_eq!(gs.message_number, 0);
        assert_eq!(gs.p2_turn, true);
    }

    #[test]
    fn test_from_request() {
        let r = u32::new_data_request(true);
        let gs = GameState::from_request(r);
        assert!(gs.is_some());

        let gs = gs.unwrap();
        assert_eq!(gs.board, [0u8; 9]);
        assert_eq!(gs.turn, 0);
        assert_eq!(gs.message_number, 0);
        assert_eq!(gs.p2_turn, false);
    }

    #[test]
    fn test_from_request_p2_turn() {
        let mut r = u32::new_data_request(false);
        r = r ^ (1 << Bits::P2Turn as u32) | (1 << Bits::MessageNumber as u32) | (1 << Bits::TurnOffset as u32);
        let gs = GameState::from_request(r);
        assert!(gs.is_some());
        let gs = gs.unwrap();
        assert_eq!(gs.board, [0u8; 9]);
        assert_eq!(gs.turn, 1);
        assert_eq!(gs.message_number, 1);
        assert_eq!(gs.p2_turn, true);
    }

    #[test]
    fn test_from_request_board_all_ones() {
        let r = 0b111111111;
        let gs = GameState::from_request(r);
        assert!(gs.is_some());
        let gs = gs.unwrap();
        assert_eq!(gs.board, [1u8; 9]);
        assert_eq!(gs.turn, 0);
        assert_eq!(gs.message_number, 0);
        assert_eq!(gs.p2_turn, false);
    }

    #[test]
    fn test_from_request_invalid_turn() {
        let r = (1 << Bits::TurnOffset as u32) | (1 << Bits::MessageNumber as u32);
        let gs = GameState::from_request(r);
        assert!(gs.is_none());
    }
    #[test]
    fn test_from_request_invalid_player() {
        let r = 1 << Bits::P2Turn as u32;
        let gs = GameState::from_request(r);
        assert!(gs.is_none());
    }

    #[test]
    fn test_p2_turn() {
        let gs = GameState::new();
        assert_eq!(gs.p2_turn(), true);
    }

    #[test]
    fn test_compare_boards() {
        let mut gs = GameState::new();
        let mut gs2 = GameState::new();
        // This is false because no changes have been made, you can't pass your turn in tic tac toe
        assert_eq!(gs.compare_boards(&gs2), false);
        gs2.board[0] = 1;
        assert_eq!(gs.compare_boards(&gs2), true);
        gs.board[0] = 1;
        gs2.board[0] = 2;
        assert_eq!(gs.compare_boards(&gs2), false);
    }
}