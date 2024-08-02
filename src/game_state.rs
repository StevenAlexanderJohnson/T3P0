use crate::{
    request::{DataRequest, Request},
    Player, PlayerTrait,
};

#[derive(Debug, Clone)]
pub struct GameState {
    players: Option<Box<[Player; 2]>>,
    submitted_by: Player,
    board: [u8; 9],
    turn: u8,
    message_number: u8,
    p2_turn: bool,
    request: Request,
}

impl GameState {}

pub trait GameStateTrait {
    fn new(player: Option<Player>, players: Option<[Player; 2]>) -> Self;
    fn from_request(request: Request, player: Player) -> Result<Self, &'static str>
    where
        Self: Sized;
    fn compare_boards(&self, other: &GameState) -> bool;
    fn validate_turn(&self, game_state: &Self) -> Result<bool, &'static str>;
    fn to_request(&self) -> Request;
}

impl GameStateTrait for GameState {
    fn new(player: Option<Player>, players: Option<[Player; 2]>) -> Self {
        GameState {
            players: match players {
                Some(p) => Some(Box::new(p)),
                None => None,
            },
            submitted_by: match player {
                Some(p) => p,
                None => Player::new(),
            },
            turn: 0,
            p2_turn: true,
            message_number: 0,
            board: [0u8; 9],
            request: Request::new_data_request(false),
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
    fn from_request(request: Request, player: Player) -> Result<Self, &'static str> {
        request.validate_request()?;

        let mut board = [0u8; 9];
        let board_state = request.get_board_state();
        for (i, item) in board.iter_mut().enumerate() {
            *item = (board_state >> i) as u8 & 1;
        }

        Ok(GameState {
            players: None,
            submitted_by: player,
            board,
            turn: request.get_turn(),
            message_number: request.get_message_number(),
            p2_turn: request.get_is_p2_turn(),
            request,
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
    /// This value is going to come from the TCP connection.
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
        // If the new game state is submitted by the same player, it is not a valid turn
        if self.submitted_by.get_id() == game_state.submitted_by.get_id() {
            return Ok(false);
        }
        // Check if the new game state submitted by is one of the players
        if self.players.is_some()
            && !self
                .players
                .as_ref()
                .unwrap()
                .contains(&game_state.submitted_by)
        {
            return Ok(false);
        }

        if !self.compare_boards(game_state) {
            return Ok(false);
        }

        Ok(true)
    }

    fn to_request(&self) -> Request {
        self.request
    }
}

#[cfg(test)]
mod game_state_test {
    use super::*;
    use crate::request::{Bits, DataRequest, Request};

    #[test]
    fn test_new() {
        let gs = GameState::new(None, Some([Player::new(), Player::new()]));
        assert_eq!(gs.board, [0u8; 9]);
        assert_eq!(gs.turn, 0);
        assert_eq!(gs.message_number, 0);
        assert_eq!(gs.p2_turn, true);
    }

    #[test]
    fn test_from_request() {
        let r = Request::new_data_request(true);
        let gs = GameState::from_request(r, Player::new());
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
        let gs = GameState::from_request(r, Player::new());
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
        let gs = GameState::from_request(r, Player::new());
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
        let gs = GameState::from_request(r, Player::new());
        assert!(gs.is_err());
    }
    #[test]
    fn test_from_request_invalid_player() {
        let r = Request(1 << Bits::P2Turn as u32);
        let gs = GameState::from_request(r, Player::new());
        assert!(gs.is_err());
    }

    #[test]
    fn test_compare_boards() {
        let players = [Player::new(), Player::new()];
        let mut gs = GameState::new(None, Some(players.clone()));
        let mut gs2 = GameState::new(None, Some(players.clone()));
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
        let players = [Player::new(), Player::new()];
        let mut gs = GameState::new(None, Some(players.clone()));
        gs.turn = 0;
        gs.message_number = 0;
        gs.p2_turn = false;
        gs.submitted_by = players[0].clone();

        let mut gs2 = GameState::new(None, Some(players.clone()));
        gs2.turn = 1;
        gs2.message_number = 1;
        gs2.p2_turn = true;
        gs2.submitted_by = players[1].clone();
        gs2.board = [1u8, 0, 0, 0, 0, 0, 0, 0, 0];

        assert!(gs.validate_turn(&gs2).is_ok());
        assert_eq!(gs.validate_turn(&gs2).unwrap(), true);
    }

    #[test]
    fn test_invalid_turn_number() {
        let players = [Player::new(), Player::new()];
        let mut gs = GameState::new(None, Some(players.clone()));
        gs.turn = 2;
        gs.message_number = 1;
        gs.p2_turn = false;
        gs.submitted_by = players[0].clone();

        let mut gs2 = GameState::new(None, Some(players.clone()));
        gs2.turn = 0;
        gs2.message_number = 0;
        gs2.p2_turn = true;
        gs2.submitted_by = players[1].clone();

        assert_eq!(gs.validate_turn(&gs2).unwrap(), false);
    }

    #[test]
    fn test_invalid_message_number() {
        let players = [Player::new(), Player::new()];
        let mut gs = GameState::new(None, Some(players.clone()));
        gs.turn = 1;
        gs.message_number = 2;
        gs.p2_turn = false;
        gs.submitted_by = players[0].clone();

        let mut gs2 = GameState::new(None, Some(players.clone()));
        gs2.turn = 0;
        gs2.message_number = 0;
        gs2.p2_turn = true;
        gs2.submitted_by = players[1].clone();

        assert_eq!(gs.validate_turn(&gs2).unwrap(), false);
    }

    #[test]
    fn test_invalid_same_player_turn() {
        let players = [Player::new(), Player::new()];
        let mut gs = GameState::new(None, Some(players.clone()));
        gs.turn = 1;
        gs.message_number = 1;
        gs.p2_turn = true;
        gs.submitted_by = players[0].clone();

        let mut gs2 = GameState::new(None, Some(players.clone()));
        gs2.turn = 0;
        gs2.message_number = 0;
        gs2.p2_turn = true;
        gs2.submitted_by = players[0].clone();

        assert_eq!(gs.validate_turn(&gs2).unwrap(), false);
    }

    #[test]
    fn test_invalid_submitted_by_not_player() {
        let players = [Player::new(), Player::new()];
        let mut gs = GameState::new(None, Some(players.clone()));
        gs.turn = 1;
        gs.message_number = 1;
        gs.p2_turn = false;
        gs.submitted_by = Player::new();

        let mut gs2 = GameState::new(None, Some(players.clone()));
        gs2.turn = 0;
        gs2.message_number = 0;
        gs2.p2_turn = true;
        gs2.submitted_by = players[0].clone();

        assert_eq!(gs.validate_turn(&gs2).unwrap(), false);
    }
}
