use std::u32;

use t3p0::request::DataRequest;

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

    let mut r = u32::new_data_request(false);
    println!("0: Message -> {}, Turn -> {}, Player 2 -> {}, {:#034b}", r.get_message_number(), r.get_turn(), r.get_is_p2_turn(), r);
    for i in 0..29 {
        r = match r.increment_turn_and_message() {
            Ok(r) => r,
            Err(e) => {
                println!("{}", e);
                break;
            }
        };

        println!("{}: Message -> {}, Turn -> {}, Player 2 -> {}, {:#034b}", i+1, r.get_message_number(), r.get_turn(), r.get_is_p2_turn(), r);
    }
}