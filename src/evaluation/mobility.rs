use crate::board::Board;
use crate::types::{Color, Piece};

#[rustfmt::skip]
const MOBILITY: [&[(i32, i32)]; Piece::NUM] = [
    &[],
    &[],
    &[  // Bishop
        (  0,   0), (-47, -57), (-34, -44), (-26, -11), (-14,  -2), ( -8,   7), (  4,  23),
        ( 13,  29), ( 20,  40), ( 21,  44), ( 26,  51), ( 29,  47), ( 30,  47), ( 57,  37),
    ],
    &[  // Rook
        (  0,   0), (  0,   0), (-85,  43), (-76,  56), (-69,  63), (-64,  66), (-61,  70),
        (-58,  75), (-52,  77), (-45,  80), (-38,  84), (-31,  86), (-26,  90), (-17,  93),
        (-13,  93),
    ],
    &[  // Queen
        (  0,   0), (  0,   0), (  0,   0), ( -7,  -1), (-31, -27), (  5, -24), (  5,   2),
        (  3,  73), (  8,  90), (  8, 111), ( 12, 118), ( 15, 130), ( 17, 142), ( 20, 145),
        ( 24, 151), ( 24, 161), ( 25, 168), ( 27, 176), ( 27, 183), ( 28, 189), ( 30, 198),
        ( 33, 198), ( 35, 201), ( 45, 198), ( 49, 195), ( 69, 189), (125, 156), (132, 167),
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
