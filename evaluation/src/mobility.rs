use game::{lookup, Bitboard, Board, Color, Piece, Score, Square};

#[rustfmt::skip]
const MOBILITY: [&[(i32, i32)]; Piece::NUM] = [
    &[],
    &[  // Knight
        (-16, -16), (-12, -12), ( -8,  -8), ( -4,  -4), (  0,   0), (  4,   4), (  8,   8),
        ( 12,  12), ( 16,  16),
    ],
    &[  // Bishop
        (-30, -30), (-25, -25), (-20, -20), (-15, -15), (-10, -10), ( -5,  -5), (  0,   0),
        (  5,   5), ( 10,  10), ( 15,  15), ( 20,  20), ( 25,  25), ( 30,  30), ( 35,  35),
    ],
    &[  // Rook
        (-14, -14), (-12, -12), (-10, -10), ( -8,  -8), ( -6,  -6), ( -4,  -4), ( -2,  -2),
        (  0,   0), (  2,   2), (  4,   4), (  6,   6), (  8,   8), ( 10,  10), ( 12,  12),
        ( 14,  14),
    ],
    &[  // Queen
        (-20, -20), (-18, -18), (-16, -16), (-14, -14), (-12, -12), (-10, -10), ( -8,  -8),
        ( -6,  -6), ( -4,  -4), ( -2,  -2), (  0,   0), (  2,   2), (  4,   4), (  6,   6),
        (  8,   8), ( 10,  10), ( 12,  12), ( 14,  14), ( 16,  16), ( 18,  18), ( 20,  20),
        ( 22,  22), ( 24,  24), ( 26,  26), ( 28,  28), ( 30,  30), ( 32,  32), ( 34,  34),
    ],
    &[],
];

/// Evaluates the mobility difference between the two players.
///
/// The player's mobility depends on the number of squares that their pieces can move to.
/// It can also be thought of as a square control.
pub fn evaluate(board: &Board) -> (Score, Score) {
    let occupancies = board.them() | board.us();
    let (mut mg, mut eg) = (0, 0);

    for piece in [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
        for square in board.of(piece, Color::White) {
            let count = get_attacks(square, piece, occupancies).count() as usize;
            let (mg_inc, eg_inc) = MOBILITY[piece][count];
            mg += mg_inc;
            eg += eg_inc;
        }

        for square in board.of(piece, Color::Black) {
            let count = get_attacks(square, piece, occupancies).count() as usize;
            let (mg_inc, eg_inc) = MOBILITY[piece][count];
            mg -= mg_inc;
            eg -= eg_inc;
        }
    }

    (Score(mg), Score(eg))
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
