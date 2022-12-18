mod perft;

use game::board::Board;

fn main() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    perft::run(6, &mut Board::from_fen(fen).unwrap());
}
