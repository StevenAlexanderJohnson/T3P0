use std::u32;

mod request;
mod game_state;

use request::DataRequest;

fn main() {
    let bitmask: u32 = u32::MAX;
    println!("Bitmask: {:#034b}", bitmask);

    let r = u32::MAX;
    let turn = r.get_turn();
    println!("Turn: {:#034b}, {}", turn, turn);
    println!("Is P2 Turn: {}", r.get_is_p2_turn());
    println!("Swapping Players: {:#034b}", r.swap_player());
    let r = r.swap_player();
    println!("Is p2 Turn: {}", r.get_is_p2_turn());

    println!("{}", r.get_message_number());

    let board_state = r.get_board_state();
    println!("{:#034b}, {}", board_state, board_state);
}