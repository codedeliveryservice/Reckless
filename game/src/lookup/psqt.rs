//! It's usually better to give bonuses for pieces that are well placed and penalties for pieces that are badly placed.
//! Other squares will have a neutral value of 0.
//!
//! See [Simplified Evaluation Function](https://www.chessprogramming.org/Simplified_Evaluation_Function#Piece-Square_Tables)
//! for more information.

use crate::attacks::Color;

const FILES: usize = 8 / 2;
const RANKS: usize = 8;

/// Symmetrically distributed piece-square bonuses (mirrored along the Y axis).
#[rustfmt::skip]
const BONUSES: [[[(i32, i32); FILES]; RANKS]; 6] = [
    [   // Pawn
        [(0, 0); FILES]; RANKS
    ],
    [   // Knight
        [(-50, 0), (-40, 0), (-30, 0), (-30, 0)],
        [(-40, 0), (-20, 0), (  0, 0), (  0, 0)],
        [(-30, 0), (  0, 0), ( 10, 0), ( 15, 0)],
        [(-30, 0), (  5, 0), ( 15, 0), ( 20, 0)],
        [(-30, 0), (  0, 0), ( 15, 0), ( 20, 0)],
        [(-30, 0), (  5, 0), ( 10, 0), ( 15, 0)],
        [(-40, 0), (-20, 0), (  0, 0), (  5, 0)],
        [(-50, 0), (-40, 0), (-30, 0), (-30, 0)],
    ],
    [   // Bishop
        [(-20, 0), (-10, 0), (-10, 0), (-10, 0)],
        [(-10, 0), (  0, 0), (  0, 0), (  0, 0)],
        [(-10, 0), (  0, 0), (  5, 0), ( 10, 0)],
        [(-10, 0), (  5, 0), (  5, 0), ( 10, 0)],
        [(-10, 0), (  0, 0), ( 10, 0), ( 10, 0)],
        [(-10, 0), ( 10, 0), ( 10, 0), ( 10, 0)],
        [(-10, 0), (  5, 0), (  0, 0), (  0, 0)],
        [(-20, 0), (-10, 0), (-10, 0), (-10, 0)],
    ],
    [   // Rook
        [(  0, 0), (  0, 0), (  0, 0), (  0, 0)],
        [(  5, 0), ( 10, 0), ( 10, 0), ( 10, 0)],
        [( -5, 0), (  0, 0), (  0, 0), (  0, 0)],
        [( -5, 0), (  0, 0), (  0, 0), (  0, 0)],
        [( -5, 0), (  0, 0), (  0, 0), (  0, 0)],
        [( -5, 0), (  0, 0), (  0, 0), (  0, 0)],
        [( -5, 0), (  0, 0), (  0, 0), (  0, 0)],
        [(  0, 0), (  0, 0), (  0, 0), (  5, 0)],
    ],
    [   // Queen
        [(-20, 0), (-10, 0), (-10, 0), ( -5, 0)],
        [(-10, 0), (  0, 0), (  0, 0), (  0, 0)],
        [(-10, 0), (  0, 0), (  5, 0), (  5, 0)],
        [( -5, 0), (  0, 0), (  5, 0), (  5, 0)],
        [( -5, 0), (  0, 0), (  5, 0), (  5, 0)],
        [(-10, 0), (  5, 0), (  5, 0), (  5, 0)],
        [(-10, 0), (  0, 0), (  5, 0), (  0, 0)],
        [(-20, 0), (-10, 0), (-10, 0), ( -5, 0)],
    ],
    [   // King
        [(0, 0); FILES]; RANKS
    ]
];

/// Asymmetrically distributed pawn bonuses.
#[rustfmt::skip]
const PAWN_BONUSES: [(i32, i32); 64] = [
    ( 0, 0), ( 0, 0), (  0, 0), (  0, 0), (  0, 0), (  0, 0), ( 0, 0), ( 0, 0),
    (50, 0), (50, 0), ( 50, 0), ( 50, 0), ( 50, 0), ( 50, 0), (50, 0), (50, 0),
    (10, 0), (10, 0), ( 20, 0), ( 30, 0), ( 30, 0), ( 20, 0), (10, 0), (10, 0),
    ( 5, 0), ( 5, 0), ( 10, 0), ( 25, 0), ( 25, 0), ( 10, 0), ( 5, 0), ( 5, 0),
    ( 0, 0), ( 0, 0), (  0, 0), ( 20, 0), ( 20, 0), (  0, 0), ( 0, 0), ( 0, 0),
    ( 5, 0), (-5, 0), (-10, 0), (  0, 0), (  0, 0), (-10, 0), (-5, 0), ( 5, 0),
    ( 5, 0), (10, 0), ( 10, 0), (-20, 0), (-20, 0), ( 10, 0), (10, 0), ( 5, 0),
    ( 0, 0), ( 0, 0), (  0, 0), (  0, 0), (  0, 0), (  0, 0), ( 0, 0), ( 0, 0),
];

/// Generates a piece-square table indexed by [color][piece][square].
pub fn generate_map() -> [[[(i32, i32); 64]; 6]; 2] {
    let mut map = [[[(0, 0); 64]; 6]; 2];

    for (square, &pawn_bonus) in PAWN_BONUSES.iter().enumerate() {
        map[Color::Black as usize][0][square] = (-pawn_bonus.0, -pawn_bonus.1);
        map[Color::White as usize][0][square ^ 56] = pawn_bonus;

        for (piece, &bonus) in BONUSES.iter().enumerate().skip(1) {
            let (rank, file) = (square / 8, square % 8);
            let file = if file > 3 { 7 - file } else { file };

            let bonus = bonus[rank][file];
            map[Color::Black as usize][piece][square] = (-bonus.0, -bonus.1);
            map[Color::White as usize][piece][square ^ 56] = bonus;
        }
    }

    map
}
