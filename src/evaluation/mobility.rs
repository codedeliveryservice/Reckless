use crate::types::{Bitboard, Color, Piece, Square};
use crate::{board::Board, lookup};

#[rustfmt::skip]
const MOBILITY: [&[(i32, i32)]; Piece::NUM] = [
    &[],
    &[  // Knight
        (  0,   0), (  0,   0), (-94,   8), (-13,   4), ( -9,  23), (  0,   0), (  3,  50),
        (  8,  60), ( 15,  73),
    ],
    &[  // Bishop
        (  0,   0), (-21, -97), (-19, -78), (-15, -36), ( -5, -18), ( -3,  -2), (  5,  17),
        ( 12,  28), ( 16,  39), ( 14,  47), ( 15,  53), ( 18,  46), ( 29,  49), ( 44,  49),
    ],
    &[  // Rook
        (  0,   0), (  0,   0), (-23,   4), (-20,  17), (-19,  25), (-18,  35), (-18,  42),
        (-17,  45), (-12,  49), ( -8,  50), ( -2,  57), (  0,  61), (  5,  61), ( 13,  59),
        ( 10,  58),
    ],
    &[  // Queen
        (  0,   0), (  0,   0), (  0,   0), ( 32, -95), (-14, -83), ( 19, -49), ( 22, -31),
        ( 22, -21), ( 21, -12), ( 22,  -7), ( 21,  17), ( 23,  33), ( 25,  41), ( 27,  47),
        ( 27,  60), ( 28,  67), ( 29,  77), ( 27,  90), ( 29,  91), ( 29,  93), ( 30,  98),
        ( 24, 110), ( 28, 112), ( 15, 127), ( 13, 125), ( 51, 109), ( 87, 108), ( 96, 105),
    ],
    &[],
];

/// Evaluates the mobility difference between the two players.
///
/// The player's mobility depends on the number of squares that their pieces can move to.
/// It can also be thought of as a square control.
pub fn evaluate(board: &Board) -> (i32, i32) {
    let (mut mg, mut eg) = (0, 0);

    for piece in [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
        for square in board.of(piece, Color::White) {
            let count = get_attacks(square, piece, board.occupancies()).count() as usize;
            let (mg_inc, eg_inc) = MOBILITY[piece][count];
            mg += mg_inc;
            eg += eg_inc;
        }

        for square in board.of(piece, Color::Black) {
            let count = get_attacks(square, piece, board.occupancies()).count() as usize;
            let (mg_inc, eg_inc) = MOBILITY[piece][count];
            mg -= mg_inc;
            eg -= eg_inc;
        }
    }

    (mg, eg)
}

fn get_attacks(square: Square, piece: Piece, occupancies: Bitboard) -> Bitboard {
    match piece {
        Piece::Knight => lookup::knight_attacks(square),
        Piece::Bishop => lookup::bishop_attacks(square, occupancies),
        Piece::Rook => lookup::rook_attacks(square, occupancies),
        Piece::Queen => lookup::queen_attacks(square, occupancies),
        _ => panic!("Invalid piece"),
    }
}
