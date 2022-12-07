use game::board::Board;

fn main() {
    let fen = "8/8/8/3nk3/8/3Q4/7p/7K w - - 0 1";
    let board = Board::from_fen(fen).unwrap();

    for m in board.generate_moves() {
        println!(
            "{:?} {:?} (capture: {})",
            m.start(),
            m.target(),
            m.is_capture()
        );
    }
}
