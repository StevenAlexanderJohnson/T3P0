use crate::request::{DataRequest, Request};

pub struct GameState {
    board: [u8; 9],
    turn: u8,
    message_number: u8,
    p2_turn: bool
}

pub trait GameStateTrait {
    fn new() -> Self;
    fn from_request(request: Request) -> Result<Self, &'static str> where Self: Sized;
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
    fn from_request(request: Request) -> Result<Self, &'static str> {
        request.validate_request()?;

        let mut board = [0u8; 9];
        let board_state = request.get_board_state();
        for (i, item) in board.iter_mut().enumerate() {
            *item = (board_state >> i) as u8 & 1;
        }

        Ok(GameState {
            board, 
            turn: request.get_turn(),
            message_number: request.get_message_number(),
            p2_turn: request.get_is_p2_turn(),
        })
    }

    /// Returns if it is player 2's turn
    /// 
    /// # Returns
    /// 
    /// * `bool` - True if it is player 2's turn, false otherwise
    fn p2_turn(&self) -> bool {
        self.p2_turn
    }
    
    /// Compare two boards to see if they are valid moves.
    /// A valid move is when only one square is changed from the previous board.
    /// If the board is changing a value that is already changed, it is not a valid move.
    /// 
    /// # Arguments
    /// 
    /// * `other` - The other GameState to compare to
    /// 
    /// # Returns
    /// 
    /// * `bool` - True if the boards are valid moves, false otherwise
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
mod game_state_test {
    use crate::game_state::{GameState, GameStateTrait};
    use crate::request::{Bits, DataRequest, Request};

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
        let r = Request::new_data_request(true);
        let gs = GameState::from_request(r);
        assert!(gs.is_ok());

        let gs = gs.unwrap();
        assert_eq!(gs.board, [0u8; 9]);
        assert_eq!(gs.turn, 0);
        assert_eq!(gs.message_number, 0);
        assert_eq!(gs.p2_turn, false);
    }

    #[test]
    fn test_from_request_p2_turn() {
        let mut r = Request::new_data_request(false);
        r = Request(r.0 ^ (1 << Bits::P2Turn as u32) | (1 << Bits::MessageNumber as u32) | (1 << Bits::TurnOffset as u32));
        let gs = GameState::from_request(r);
        assert!(gs.is_ok());
        let gs = gs.unwrap();
        assert_eq!(gs.board, [0u8; 9]);
        assert_eq!(gs.turn, 1);
        assert_eq!(gs.message_number, 1);
        assert_eq!(gs.p2_turn, true);
    }

    #[test]
    fn test_from_request_board_all_ones() {
        let r = Request(0b111111111);
        let gs = GameState::from_request(r);
        assert!(gs.is_ok());
        let gs = gs.unwrap();
        assert_eq!(gs.board, [1u8; 9]);
        assert_eq!(gs.turn, 0);
        assert_eq!(gs.message_number, 0);
        assert_eq!(gs.p2_turn, false);
    }

    #[test]
    fn test_from_request_invalid_turn() {
        let r = Request((1 << Bits::TurnOffset as u32) | (1 << Bits::MessageNumber as u32));
        let gs = GameState::from_request(r);
        assert!(gs.is_err());
    }
    #[test]
    fn test_from_request_invalid_player() {
        let r = Request(1 << Bits::P2Turn as u32);
        let gs = GameState::from_request(r);
        assert!(gs.is_err());
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