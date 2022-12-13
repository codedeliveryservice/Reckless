use game::board::Board;

pub fn perft(board: &mut Board, depth: u32) -> u32 {
    if depth == 0 {
        return 1;
    }

    let mut nodes = 0;

    for mv in board.generate_moves() {
        if board.make_move(mv).is_ok() {
            nodes += perft(board, depth - 1);
            board.take_back();
        }
    }

    nodes
}

macro_rules! assert_perft {
        ($($name:ident, $fen:tt, $depth:tt, $expected:tt;)*) => {$(
            #[test]
            fn $name() {
                let actual = common::perft(&mut game::board::Board::from_fen($fen).unwrap(), $depth);
                assert_eq!(actual, $expected)
            }
        )*};
    }

pub(crate) use assert_perft;
