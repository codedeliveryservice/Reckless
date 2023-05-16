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
const BONUSES: [[[i32; FILES]; RANKS]; 6] = [
    [   // Pawn
        [0; FILES]; RANKS
    ],
    [   // Knight
        [-50, -40, -30, -30],
        [-40, -20,   0,   0],
        [-30,   0,  10,  15],
        [-30,   5,  15,  20],
        [-30,   0,  15,  20],
        [-30,   5,  10,  15],
        [-40, -20,   0,   5],
        [-50, -40, -30, -30],
    ],
    [   // Bishop
        [-20, -10, -10, -10],
        [-10,   0,   0,   0],
        [-10,   0,   5,  10],
        [-10,   5,   5,  10],
        [-10,   0,  10,  10],
        [-10,  10,  10,  10],
        [-10,   5,   0,   0],
        [-20, -10, -10, -10],
    ],
    [   // Rook
        [  0,   0,   0,   0],
        [  5,  10,  10,  10],
        [ -5,   0,   0,   0],
        [ -5,   0,   0,   0],
        [ -5,   0,   0,   0],
        [ -5,   0,   0,   0],
        [ -5,   0,   0,   0],
        [  0,   0,   0,   5],
    ],
    [   // Queen
        [-20, -10, -10,  -5],
        [-10,   0,   0,   0],
        [-10,   0,   5,   5],
        [ -5,   0,   5,   5],
        [ -5,   0,   5,   5],
        [-10,   5,   5,   5],
        [-10,   0,   5,   0],
        [-20, -10, -10,  -5],
    ],
    [   // King
        [0; FILES]; RANKS
    ]
];

/// Asymmetrically distributed pawn bonuses.
#[rustfmt::skip]
const PAWN_BONUSES: [i32; 64] = [
     0,   0,   0,   0,   0,   0,   0,   0,
    50,  50,  50,  50,  50,  50,  50,  50,
    10,  10,  20,  30,  30,  20,  10,  10,
     5,   5,  10,  25,  25,  10,   5,   5,
     0,   0,   0,  20,  20,   0,   0,   0,
     5,  -5, -10,   0,   0, -10,  -5,   5,
     5,  10,  10, -20, -20,  10,  10,   5,
     0,   0,   0,   0,   0,   0,   0,   0,
];

/// Generates a piece-square table indexed by [color][piece][square].
pub fn generate_map() -> [[[i32; 64]; 6]; 2] {
    let mut map = [[[0; 64]; 6]; 2];

    for (square, &pawn_bonus) in PAWN_BONUSES.iter().enumerate() {
        map[Color::Black as usize][0][square] = -pawn_bonus;
        map[Color::White as usize][0][square ^ 56] = pawn_bonus;

        for (piece, &bonus) in BONUSES.iter().enumerate().skip(1) {
            let (rank, file) = (square / 8, square % 8);
            let file = if file > 3 { 7 - file } else { file };

            map[Color::Black as usize][piece][square] = -bonus[rank][file];
            map[Color::White as usize][piece][square ^ 56] = bonus[rank][file];
        }
    }

    map
}
