//! It's usually better to give bonuses for pieces that are well placed and penalties for pieces that are badly placed.
//! Other squares will have a neutral value of 0.
//!
//! See [Simplified Evaluation Function](https://www.chessprogramming.org/Simplified_Evaluation_Function#Piece-Square_Tables)
//! for more information.

use game::{Board, Color, Piece, Score, Square};

#[rustfmt::skip]
const PAWNS: [i32; Square::NUM] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    50, 50, 50, 50, 50, 50, 50, 50,
    10, 10, 20, 30, 30, 20, 10, 10,
     5,  5, 10, 25, 25, 10,  5,  5,
     0,  0,  0, 20, 20,  0,  0,  0,
     5, -5,-10,  0,  0,-10, -5,  5,
     5, 10, 10,-20,-20, 10, 10,  5,
     0,  0,  0,  0,  0,  0,  0,  0,
];

#[rustfmt::skip]
const KNIGHTS: [i32; Square::NUM] = [
    -50,-40,-30,-30,-30,-30,-40,-50,
    -40,-20,  0,  0,  0,  0,-20,-40,
    -30,  0, 10, 15, 15, 10,  0,-30,
    -30,  5, 15, 20, 20, 15,  5,-30,
    -30,  0, 15, 20, 20, 15,  0,-30,
    -30,  5, 10, 15, 15, 10,  5,-30,
    -40,-20,  0,  5,  5,  0,-20,-40,
    -50,-40,-30,-30,-30,-30,-40,-50,
];

#[rustfmt::skip]
const BISHOPS: [i32; Square::NUM] = [
    -20,-10,-10,-10,-10,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5, 10, 10,  5,  0,-10,
    -10,  5,  5, 10, 10,  5,  5,-10,
    -10,  0, 10, 10, 10, 10,  0,-10,
    -10, 10, 10, 10, 10, 10, 10,-10,
    -10,  5,  0,  0,  0,  0,  5,-10,
    -20,-10,-10,-10,-10,-10,-10,-20,
];

#[rustfmt::skip]
const ROOKS: [i32; Square::NUM] = [
      0,  0,  0,  0,  0,  0,  0,  0,
      5, 10, 10, 10, 10, 10, 10,  5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
     -5,  0,  0,  0,  0,  0,  0, -5,
      0,  0,  0,  5,  5,  0,  0,  0,
];

#[rustfmt::skip]
const QUEENS: [i32; Square::NUM] = [
    -20,-10,-10, -5, -5,-10,-10,-20,
    -10,  0,  0,  0,  0,  0,  0,-10,
    -10,  0,  5,  5,  5,  5,  0,-10,
     -5,  0,  5,  5,  5,  5,  0, -5,
      0,  0,  5,  5,  5,  5,  0, -5,
    -10,  5,  5,  5,  5,  5,  0,-10,
    -10,  0,  5,  0,  0,  0,  0,-10,
    -20,-10,-10, -5, -5,-10,-10,-20,
];

#[rustfmt::skip]
const MIRRORED: [usize; Square::NUM] = [
    56, 57, 58, 59, 60, 61, 62, 63,
    48, 49, 50, 51, 52, 53, 54, 55,
    40, 41, 42, 43, 44, 45, 46, 47,
    32, 33, 34, 35, 36, 37, 38, 39,
    24, 25, 26, 27, 28, 29, 30, 31,
    16, 17, 18, 19, 20, 21, 22, 23,
     8,  9, 10, 11, 12, 13, 14, 15,
     0,  1,  2,  3,  4,  5,  6,  7,
];

#[rustfmt::skip]
const LOCATION_SCORES: [LocationScore; 5] = [
    LocationScore { piece: Piece::Pawn,   table: PAWNS },
    LocationScore { piece: Piece::Knight, table: KNIGHTS },
    LocationScore { piece: Piece::Bishop, table: BISHOPS },
    LocationScore { piece: Piece::Rook,   table: ROOKS },
    LocationScore { piece: Piece::Queen,  table: QUEENS },
];

struct LocationScore {
    piece: Piece,
    table: [i32; Square::NUM],
}

pub fn evaluate_locations(board: &Board) -> Score {
    get_score_for_white(board) - get_score_for_black(board)
}

fn get_score_for_white(board: &Board) -> Score {
    let mut score = 0;
    for pair in LOCATION_SCORES {
        for square in board.of(pair.piece, Color::White) {
            score += pair.table[MIRRORED[square.0 as usize]];
        }
    }
    Score::new(score)
}

fn get_score_for_black(board: &Board) -> Score {
    let mut score = 0;
    for pair in LOCATION_SCORES {
        for square in board.of(pair.piece, Color::Black) {
            score += pair.table[square.0 as usize];
        }
    }
    Score::new(score)
}
