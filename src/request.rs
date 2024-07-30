// A 32 bit unsigned integer is going to be used to represent the request/response.

// There are nine spots in a tic tac toe board, so 9 bits are needed to represent the board.
// The board is going to be represented as a grid as follows
//  0 | 1 | 2
// -----------
//  3 | 4 | 5
// -----------
//  6 | 7 | 8
// The nubmers represent the bit offset from the least significant bit.
// For example, 0x000000001 is the top left corner and 0x100000000 is the bottom right corner.

// How do we represent the board state if there are three possible states, empty, X, and O?
// The server should send the board state as the opponent sees it.

/// |----|--------------|
/// | 1  | Message Type | There are two possible message types. Data and Ok.
/// |----|--------------|
/// | 2  | Turn Number  |
/// | 3  |              | Turn number uses 4 buts for a max of 16 possible moves.
/// | 4  |              | It only takes 9 at max for a game but 3 bits is too few.
/// | 5  |              |
/// |----|--------------|
/// | 6  | Is P2 Turn   |
/// |----|--------------|
/// | 7  |Message Number|
/// | 8  |              |
/// | 9  |              | 5 bits gives (2^5)-1 possible moves which is 31 possible moves.
/// | 10 |              | This opens the possibility of best of 3s which will use at most 27.
/// | 11 |              |
/// |----|--------------|
/// | 12 | Unused       |
/// | 13 |              |
/// | 14 |              |
/// | 15 |              |
/// | 16 |              |
/// | 17 |              |
/// | 18 |              |
/// | 19 |              |
/// | 20 |              |
/// | 21 |              |
/// | 22 |              |
/// | 23 |              |
/// |----|--------------|
/// | 24 | Board State  |
/// | 25 |              | 0 | 1 | 2
/// | 26 |              | ---------
/// | 27 |              | 3 | 4 | 5
/// | 28 |              | ---------
/// | 29 |              | 6 | 7 | 8
/// | 30 |              | See note above the diagram that describes how this board
/// | 31 |              | is represented.
/// | 32 |              |
/// |----|--------------|

#[derive(Debug)]
#[repr(u32)]
pub enum Bits {
    // 5 bits represent the message number, supporting up to 31 messages.
    MessageNumber = 21u32,
    P2Turn = 26u32,
    // 4 bits represent the turn number. Supporting up to 15 turns, but only 9 are needed to end a game.
    TurnOffset = 27u32,
    MessageType = 1u32 << 31,
}

#[derive(Debug)]
#[repr(u32)]
pub enum Ranges {
    Board = 9u32,
    MessageNumber = 5u32,
    Turn = 4u32,
}

pub trait DataRequest {
    fn new_data_request(is_ok_response: bool) -> Self;
    fn swap_player(&self) -> Self;
    fn get_turn(&self) -> u8;
    fn get_message_number(&self) -> u8;
    fn get_board_state(&self) -> u16;
    fn get_is_p2_turn(&self) -> bool;
    fn increment_turn_and_message(&self) -> Result<Self, &'static str> where Self: Sized;
}

impl DataRequest for u32 {
    /// Creates a new u32 with formatted Ok response if chosen.
    /// If `is_ok_response` is not true then it simply returns 0.
    ///
    /// # Arguments
    ///
    /// * `is_ok_response` - A boolean to represent if this should be initialized as an Ok response.
    ///
    /// # Returns
    ///
    /// * `u32` - A response u32 with possibly initialized values.
    fn new_data_request(is_ok_response: bool) -> Self {
        if is_ok_response {
            return Bits::MessageType as u32;
        }
        0
    }

    /// Gets the turn value from the u32 request.
    ///
    /// # Returns
    ///
    /// * `u8` - A u8 that represents the current turn value.
    fn get_turn(&self) -> u8 {
        ((self >> Bits::TurnOffset as u32) & (1 << Ranges::Turn as u32) - 1) as u8
    }

    /// Gets the board state from the u32 request.
    ///
    /// # Returns
    ///
    /// * `u16` - A u16 that represents the current board state.
    /// It returns as a u16 instead of a `[u8; 9]` because I wanted the possibility to keep it as an integer.
    fn get_board_state(&self) -> u16 {
        (self & (1 << Ranges::Board as u32) - 1) as u16
    }

    /// Gets whether it's the second player's turn.
    ///
    /// # Returns
    ///
    /// * `bool` - A boolean that is true if it's player 2's turn and false if it's player 1.
    fn get_is_p2_turn(&self) -> bool {
        (self >> Bits::P2Turn as u32) & 1 == 1
    }

    /// Gets the current message number.
    ///
    /// # Returns
    ///
    /// * `u8` - A `u8` that holds the number of messages that have passed.
    /// Messages only require 5 bits but `u8` is the smallest that fits.
    fn get_message_number(&self) -> u8 {
        ((self >> Bits::MessageNumber as u32) & (1 << Ranges::MessageNumber as u32) - 1) as u8
    }

    /// Switches the bit that represents whose turn it is and flips the state of the board.
    ///
    /// # Returns
    ///
    /// * `u32` - A new u32 that represents the exact board state but it's flipped to the other users view.
    fn swap_player(&self) -> u32 {
        let mut output = *self;
        for i in 0..Ranges::Board as usize {
            output = output ^ (1 << i);
        }
        output = output ^ (1 << Bits::P2Turn as u32);
        output
    }

    fn increment_turn_and_message(&self) -> Result<Self, &'static str> {
        let turn = self.get_turn();
        let message_number = self.get_message_number();
        if message_number + 1 >= 27 {
            return Err("Trying to increment message number past maximum value.");
        }
        // First clear out that set of bits then | that number plus 1
        let mut output = self ^ (u32::from(turn) << u32::from(Bits::TurnOffset as u32)) as u32;
        output = output | (u32::from(turn) + 1) % 9 << u32::from(Bits::TurnOffset as u32) as u32;
        output = output ^ (u32::from(message_number) << Bits::MessageNumber as u32) as u32;
        output = output | (u32::from(message_number + 1) << Bits::MessageNumber as u32) as u32;
        output = output ^ (1 << Bits::P2Turn as u32);
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_request() {
        let r = u32::new_data_request(false);
        assert_eq!(r, 0);
    }

    #[test]
    fn test_new_request_from_server() {
        let r = u32::new_data_request(true);
        assert_eq!(r, Bits::MessageType as u32);
    }

    #[test]
    fn test_get_turn() {
        // All zeros should be turn 0
        let r = 0b0;
        let turn = r.get_turn();
        assert_eq!(turn, 0);
    }

    #[test]
    fn test_get_turn_first() {
        // Shifting one to the first bit in the range should be turn 1
        let r = 0b1 << Bits::TurnOffset as u32;
        let turn = r.get_turn();
        assert_eq!(turn, 1);
    }

    #[test]
    fn test_get_turn_all_ones() {
        // Shifting four bits to the first bit in the range should make the whole range 1s resulting in 15
        let r = 0b1111 << Bits::TurnOffset as u32;
        let turn = r.get_turn();
        assert_eq!(turn, 15);
    }

    #[test]
    fn test_get_turn_bounds() {
        // Shifting five bits to the first bit should make the range all 1s with an extra 1 to the left of the range.
        // This extra 1 shouldn't affect the result.
        let r = 0b11111 << Bits::TurnOffset as u32;
        let turn = r.get_turn();
        assert_eq!(turn, 15);
        // Shifting four bits to the first bit but minus 1 should make the range all 1s except the left most bit.
        let r = 0b1111 << (Bits::TurnOffset as u32 - 1);
        let turn = r.get_turn();
        assert_eq!(turn, 7);
    }

    #[test]
    fn test_get_board_state_all_zeros() {
        // All zeros should be board state 0
        let r = 0b0;
        let board_state = r.get_board_state();
        assert_eq!(board_state, 0);
    }

    #[test]
    fn test_get_board_state_all_ones() {
        // All ones should be board state 511
        let r = 0b111111111;
        let board_state = r.get_board_state();
        assert_eq!(board_state, 511);
    }

    #[test]
    fn test_get_board_state_exta_one_bit() {
        // Testing with an extra 1 to make sure that the extra 1 doesn't affect the result.
        let r = 0b1111111111;
        let board_state = r.get_board_state();
        assert_eq!(board_state, 511);
        let r = 0b1000000000;
        let board_state = r.get_board_state();
        assert_eq!(board_state, 0);
    }

    #[test]
    fn test_get_board_state_position_0() {
        // With just 1 that means that the top left is filled in.
        let r = 0b1;
        let board_state = r.get_board_state();
        assert_eq!(board_state, 1);
    }

    #[test]
    fn test_get_board_state_position_8() {
        // With just 1 that means that the bottom right is filled in.
        let r = 0b100000000;
        let board_state = r.get_board_state();
        assert_eq!(board_state, 256);
    }

    #[test]
    fn test_get_is_p2_turn_true() {
        // If the msb is 1, then it's player 1's turn
        let r = 0b1 << Bits::P2Turn as u32;
        let is_p2_turn = r.get_is_p2_turn();
        assert_eq!(is_p2_turn, true);
    }

    #[test]
    fn test_get_is_p2_turn_false() {
        // If the msb is 0, then it's player 2's turn
        let r = u32::MAX ^ (1 << Bits::P2Turn as u32);
        let is_p2_turn = r.get_is_p2_turn();
        assert_eq!(is_p2_turn, false);
    }

    #[test]
    fn test_get_message_number() {
        // All zeros should be message number 0
        let r = 0b0;
        let message_number = r.get_message_number();
        assert_eq!(message_number, 0);
    }

    #[test]
    fn test_get_message_number_one() {
        // Shifting one to the first bit in the range should be message number 1
        let r = 0b1 << Bits::MessageNumber as u32;
        let message_number = r.get_message_number();
        assert_eq!(message_number, 1);
    }

    #[test]
    fn test_get_message_number_all_ones() {
        // Shifting five bits to the first bit in the range should make the whole range 1s resulting in 31
        let r = 0b11111 << Bits::MessageNumber as u32;
        let message_number = r.get_message_number();
        assert_eq!(message_number, 31);
    }

    #[test]
    fn test_get_message_number_bounds() {
        // Shifting six bits to the first bit should make the range all 1s with an extra 1 to the left of the range.
        // This extra 1 shouldn't affect the result.
        let r = 0b111111 << Bits::MessageNumber as u32;
        let message_number = r.get_message_number();
        assert_eq!(message_number, 31);

        let r = 0b11111 << (Bits::MessageNumber as u32 - 1);
        let message_number = r.get_message_number();
        assert_eq!(message_number, 15);
    }

    #[test]
    fn test_swap_player() {
        // All zeros should be all ones
        let r = 0b0;
        let swapped = r.swap_player();
        assert_eq!(swapped, 0 | (1 << Bits::P2Turn as u32) | (1 << 9) - 1);
    }

    #[test]
    fn test_swap_player_from_all_ones() {
        // All ones should be all zeros
        let r = u32::MAX;
        let swapped = r.swap_player();
        assert_eq!(swapped, r ^ (1 << Bits::P2Turn as u32) ^ (1 << 9) - 1);
    }

    #[test]
    fn test_swap_player_turn_separate_from_board() {
        // All zeros except the msb should be all zeros except the lsb
        let r = 0b1 << Bits::P2Turn as u32;
        let swapped = r.swap_player();
        // If the only bit that was 1 was the player turn but, then it should be 0 and the board should be all 1s.
        assert_eq!(swapped, (1 << Ranges::Board as u32) - 1);
    }

    #[test]
    fn increment_turn_and_message() {
        let r = u32::new_data_request(false);
        let incremented = r.increment_turn_and_message();
        assert!(incremented.is_ok());
        let incremented = incremented.unwrap();
        assert_eq!(
            incremented,
            (r | 1 << Bits::MessageNumber as u32
                | 1 << Bits::TurnOffset as u32
                | 1 << Bits::P2Turn as u32)
        )
    }

    #[test]
    fn increment_turn_and_message_twice() {
        let r = u32::new_data_request(false);
        let incremented = r.increment_turn_and_message();
        assert!(incremented.is_ok());
        let incremented = incremented.unwrap();
        let incremented = incremented.increment_turn_and_message();
        assert!(incremented.is_ok());
        let incremented = incremented.unwrap();
        assert_eq!(
            incremented,
            (r | 2 << Bits::MessageNumber as u32 | 2 << Bits::TurnOffset as u32)
        )
    }

    #[test]
    fn increment_turn_and_message_three_times() {
        let r = u32::new_data_request(false);
        let incremented = r.increment_turn_and_message();
        assert!(incremented.is_ok());
        let incremented = incremented.unwrap();
        let incremented = incremented.increment_turn_and_message();
        assert!(incremented.is_ok());
        let incremented = incremented.unwrap();
        let incremented = incremented.increment_turn_and_message();
        assert!(incremented.is_ok());
        let incremented = incremented.unwrap();
        assert_eq!(
            incremented,
            (r | 3 << Bits::MessageNumber as u32
                | 3 << Bits::TurnOffset as u32
                | 1 << Bits::P2Turn as u32)
        )
    }

    #[test]
    fn increment_turn_and_message_turn_reset() {
        let mut r = u32::new_data_request(false);
        for _ in 0..9 {
            r = match r.increment_turn_and_message() {
                Ok(r) => r,
                Err(e) => {
                    assert_eq!(e, "Trying to increment message number past maximum value.");
                    break;
                }
            };
        }
        assert_eq!(r.get_turn(), 0);
    }

    #[test]
    fn increment_turn_and_message_past_max_message() {
        let mut r = u32::new_data_request(false);
        for _ in 0..26 {
            r = match r.increment_turn_and_message() {
                Ok(r) => r,
                Err(e) => {
                    assert_eq!(e, "Trying to increment message number past maximum value.");
                    break;
                }
            };
        }
        assert!(r.increment_turn_and_message().is_err());
    }
}
