use crate::request::{Bits, DataRequest, Request};

#[derive(Debug, Clone)]
pub struct GameState {
    board: [u8; 9],
    turn: u8,
    message_number: u8,
    p2_turn: bool,
}

impl GameState {
    pub fn validate_board(&mut self, new_state: &GameState, is_p2: bool) -> bool {
        let mut num_changes = 0;
        let mut changed_index = None;

        for i in 0..9 {
            match (self.board[i], new_state.board[i]) {
                (0, 1) => {
                    num_changes += 1;
                    changed_index = Some(i);
                }
                (p, 0) if p == if is_p2 { 2 } else { 1 } => {
                    return false;
                }
                (p1, p2) if p1 != 0 && p1 != if is_p2 { 2 } else { 1 } && p2 == 1 => {
                    return false;
                }
                _ => {}
            }
        }

        num_changes == 1 && changed_index.is_some() && self.board[changed_index.unwrap()] == 0
    }

    pub fn update_board(&mut self, new_state: &GameState, is_p2: bool) {
        for i in 0..9 {
            if new_state.board[i] != 0 {
                self.board[i] = if is_p2 { 2 } else { 1 };
            }
        }

        self.turn = new_state.turn;
        self.message_number = new_state.message_number;
        self.p2_turn = new_state.p2_turn;
    }
}

pub trait GameStateTrait {
    fn new(is_player_two: bool) -> Self;
    fn from_request(request: Request) -> Result<Self, &'static str>
    where
        Self: Sized;
    fn compare_boards(&self, other: &GameState) -> bool;
    fn validate_turn(&self, game_state: &Self) -> Result<bool, &'static str>;
    fn to_request(&self, as_ok: bool) -> Request;
    fn is_p2_turn(&self) -> bool;
}

impl GameStateTrait for GameState {
    fn new(is_player_two: bool) -> Self {
        GameState {
            turn: 0,
            p2_turn: is_player_two,
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

    /// Validate a turn to see if it is a valid move
    ///
    /// For a turn to be valid, the following conditions must be met:
    /// 1. The turn must be incremented by 1.
    /// 2. The player that submitted the new game state must be different from the player that submitted the previous game state.
    /// 3. The message number must be incremented by 1.
    /// 4. The new game state must be submitted by one of the players.
    ///     This value is going to come from the TCP connection.
    /// 5. The board must be a valid move.
    ///
    /// # Arguments
    ///
    /// * `game_state` - The next game state
    ///
    /// # Errors
    ///
    /// * `&'static str` - If the turn is not valid, the error message will describe why.
    ///
    /// # Returns
    ///
    /// * `Result<bool, &'static str>` - True if the turn is valid, false otherwise
    fn validate_turn(&self, game_state: &Self) -> Result<bool, &'static str> {
        // If the turn is not the next turn, it is not a valid turn
        if self.turn + 1 != game_state.turn {
            return Ok(false);
        }
        // If the player is the same, it is not a valid turn
        if self.p2_turn == game_state.p2_turn {
            return Ok(false);
        }
        // If the message number is not the next message number, it is not a valid turn
        if self.message_number + 1 != game_state.message_number {
            return Ok(false);
        }

        if !self.compare_boards(game_state) {
            return Ok(false);
        }

        Ok(true)
    }

    fn to_request(&self, as_ok: bool) -> Request {
        let mut output = 0u32;
        output ^= (self.turn as u32) << Bits::TurnOffset as u32
            | (self.message_number as u32) << Bits::MessageNumber as u32
            | (self.p2_turn as u32) << Bits::P2Turn as u32
            | (self.board.iter().fold(0, |acc, &x| {
                if self.p2_turn && x == 2 {
                    acc << 1 | 1
                } else {
                    acc << 1
                }
            }))
            | (self.board.iter().fold(0, |acc, &x| acc << 1 | x as u32))
            | (as_ok as u32) << Bits::MessageType as u32;

        Request(output)
    }

    fn is_p2_turn(&self) -> bool {
        self.p2_turn
    }
}

#[cfg(test)]
mod game_state_test {
    use super::*;
    use crate::request::{Bits, DataRequest, Request};

    #[test]
    fn test_new() {
        let gs = GameState::new(true);
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
        r = Request(
            r.0 ^ (1 << Bits::P2Turn as u32)
                | (1 << Bits::MessageNumber as u32)
                | (1 << Bits::TurnOffset as u32),
        );
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
    fn test_compare_boards() {
        let mut gs = GameState::new(false);
        let mut gs2 = GameState::new(true);
        // This is false because no changes have been made, you can't pass your turn in tic tac toe
        assert_eq!(gs.compare_boards(&gs2), false);
        gs2.board[0] = 1;
        assert_eq!(gs.compare_boards(&gs2), true);
        gs.board[0] = 1;
        gs2.board[0] = 2;
        assert_eq!(gs.compare_boards(&gs2), false);
    }

    // COPILOT GENERATED THESE TESTS
    // VALIDATE THEY ARE CORRECT
    #[test]
    fn test_valid_turn() {
        let mut gs = GameState::new(false);
        gs.turn = 0;
        gs.message_number = 0;

        let mut gs2 = GameState::new(true);
        gs2.turn = 1;
        gs2.message_number = 1;
        gs2.board = [1u8, 0, 0, 0, 0, 0, 0, 0, 0];

        assert!(gs.validate_turn(&gs2).is_ok());
        assert_eq!(gs.validate_turn(&gs2).unwrap(), true);
    }

    #[test]
    fn test_invalid_turn_number() {
        let mut gs = GameState::new(false);
        gs.turn = 2;
        gs.message_number = 1;

        let mut gs2 = GameState::new(true);
        gs2.turn = 0;
        gs2.message_number = 0;

        assert_eq!(gs.validate_turn(&gs2).unwrap(), false);
    }

    #[test]
    fn test_invalid_message_number() {
        let mut gs = GameState::new(false);
        gs.turn = 1;
        gs.message_number = 2;

        let mut gs2 = GameState::new(true);
        gs2.turn = 0;
        gs2.message_number = 0;

        assert_eq!(gs.validate_turn(&gs2).unwrap(), false);
    }

    #[test]
    fn test_invalid_same_player_turn() {
        let mut gs = GameState::new(true);
        gs.turn = 1;
        gs.message_number = 1;

        let mut gs2 = GameState::new(true);
        gs2.turn = 0;
        gs2.message_number = 0;
        gs2.p2_turn = true;

        assert_eq!(gs.validate_turn(&gs2).unwrap(), false);
    }

    #[test]
    fn test_invalid_submitted_by_not_player() {
        let mut gs = GameState::new(false);
        gs.turn = 1;
        gs.message_number = 1;

        let mut gs2 = GameState::new(true);
        gs2.turn = 0;
        gs2.message_number = 0;

        assert_eq!(gs.validate_turn(&gs2).unwrap(), false);
    }
}
