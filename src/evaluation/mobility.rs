use crate::board::Board;
use crate::types::{Color, Piece};

#[rustfmt::skip]
const MOBILITY: [&[(i32, i32)]; Piece::NUM] = [
    &[],
    &[],
    &[  // Bishop
        ( 0,  0), ( 0,  0), ( 0,  1), ( 0,  3), ( 2,  4), ( 4,  5), (21, 11),
        (31, 17), (38, 19), (39, 25), (43, 26), (40, 24), (26, 20), (25, 21),
    ],
    &[  // Rook
        ( 0,  0), ( 0,  0), ( 1,  5), ( 1,  5), ( 2,  7), ( 4,  8), ( 5,  8),
        ( 8,  9), (12, 11), (21, 14), (31, 20), (35, 26), (36, 34), (42, 41),
        (25, 49),
    ],
    &[  // Queen
        ( 0,  0), ( 0,  0), ( 0,  0), ( 0,  1), ( 0,  0), ( 0,  1), ( 0,  3),
        ( 0,  5), ( 1,  6), ( 1,  6), ( 3,  7), ( 5,  8), ( 7,  9), ( 9,  9),
        (14, 11), (16, 13), (18, 15), (22, 18), (24, 23), (26, 28), (30, 37),
        (31, 48), (33, 45), (36, 53), (37, 49), (41, 63), (50, 55), (51, 71),
    ],
    &[],
];

/// Evaluates the mobility difference between the two players.
///
/// The player's mobility depends on the number of squares that their pieces can move to.
/// It can also be thought of as a square control.
pub fn evaluate(board: &Board) -> (i32, i32) {
    let (mut mg, mut eg) = (0, 0);

    for piece in [Piece::Bishop, Piece::Rook, Piece::Queen] {
        for square in board.of(piece, Color::White) {
            let count = board.get_attacks(square, piece).count() as usize;
            let (mg_inc, eg_inc) = MOBILITY[piece][count];
            mg += mg_inc;
            eg += eg_inc;
        }

        for square in board.of(piece, Color::Black) {
            let count = board.get_attacks(square, piece).count() as usize;
            let (mg_inc, eg_inc) = MOBILITY[piece][count];
            mg -= mg_inc;
            eg -= eg_inc;
        }
    }

    (mg, eg)
}
