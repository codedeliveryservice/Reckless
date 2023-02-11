use game::board::Board;

pub fn perft(board: &mut Board, depth: u32) -> u32 {
    if depth == 0 {
        return 1;
    }

    let mut nodes = 0;

    for mv in board.generate_moves() {
        if board.make_move(mv).is_ok() {
            let expected_hash_key = board.generate_hash_key();
            assert_eq!(expected_hash_key, board.hash_key);
            
            nodes += perft(board, depth - 1);
            board.take_back();
        }
    }

    nodes
}

macro_rules! assert_perft {
        ($($name:ident: ($fen:tt, $depth:tt, $expected:tt),)*) => {$(
            #[test]
            fn $name() {
                let actual = common::perft(&mut game::board::Board::new($fen).unwrap(), $depth);
                assert_eq!(actual, $expected)
            }
        )*};
    }

pub(crate) use assert_perft;
