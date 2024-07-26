// Going to use a binary format for the request/response protocol.

// A 32 bit unsigned integer is going to be used to represent the request/response.

// There are nine spots in a tic tac toe board, so 9 bits are needed to represent the board.

// Four bits can represent the current turn number.

// There needs to be a way to number the requests so to avoid retry attacks.
// That's going to take at least four bits to represent 9 turns.

// How do we represent the board state if there are three possible states, empty, X, and O?
// The server should send the board state as the opponent sees it.

// That means that there are two message types:
// The user sending their move to the server, and the server sending the board state to the user.

// The board is going to be represented as a grid as follows
//  0 | 1 | 2 
// -----------
//  3 | 4 | 5 
// -----------
//  6 | 7 | 8 
// The nubmers represent the bit offset from the least significant bit.
// For example, 0x000000001 is the top left corner and 0x100000000 is the bottom right corner.


/// |----|--------------|
/// | 1  | Message Type |
/// |----|--------------|
/// | 2  | Turn Number  |
/// | 3  |              |
/// | 4  |              |
/// | 5  |              |
/// |----|--------------|
/// | 6  | Is P2 Turn   |
/// |----|--------------|
/// | 7  |Message Number|
/// | 8  |              |
/// | 9  |              |
/// | 10 |              |
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
/// | 25 |              |
/// | 26 |              |
/// | 27 |              |
/// | 28 |              |
/// | 29 |              |
/// | 30 |              |
/// | 31 |              |
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

pub trait Request {
    fn new_request(from_server: bool) -> Self;
    fn swap_player(&self) -> Self;
    fn get_turn(&self) -> u8;
    fn get_message_number(&self) -> u8;
    fn get_board_state(&self) -> u16;
    fn get_is_p2_turn(&self) -> bool;
}

impl Request for u32 {
    fn new_request(from_server: bool) -> Self {
        if from_server {
            return Bits::MessageType as u32;
        }
        0
    }

    fn get_turn(&self) -> u8 {
        ((self >> Bits::TurnOffset as u32) & (1 << Ranges::Turn as u32) - 1) as u8
    }

    fn get_board_state(&self) -> u16 {
        (self & (1 << Ranges::Board as u32) - 1) as u16
    }

    fn get_is_p2_turn(&self) -> bool {
        (self >> Bits::P2Turn as u32) & 1 == 1
    }
    
    fn get_message_number(&self) -> u8 {
        ((self >> Bits::MessageNumber as u32) & (1 << Ranges::MessageNumber as u32) - 1) as u8
    }
    
    fn swap_player(&self) -> u32 {
        let mut output = *self;
        for i in 0..Ranges::Board as usize {
            output = output ^ (1 << i);
        }
        output = output ^ (1 << Bits::P2Turn as u32);
        output
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_request() {
        let r = u32::new_request(false);
        assert_eq!(r, 0);
    }

    #[test]
    fn test_new_request_from_server() {
        let r = u32::new_request(true);
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
}