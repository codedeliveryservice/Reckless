mod perft;

use game::board::Board;

fn main() {
    perft::run(
        10,
        &mut Board::from_fen("8/8/8/8/8/1k6/8/1K6 b - - 0 1").unwrap(),
    );
}
