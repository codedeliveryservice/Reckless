mod perft;

use game::board::Board;

fn main() {
    perft::run(
        5,
        &mut Board::from_fen("8/8/2k5/3b4/4r3/8/7K/1Q6 w - - 0 1").unwrap(),
    );
}
