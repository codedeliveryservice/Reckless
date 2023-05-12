//! It's usually better to give bonuses for pieces that are well placed and penalties for pieces that are badly placed.
//! Other squares will have a neutral value of 0.
//!
//! See [Simplified Evaluation Function](https://www.chessprogramming.org/Simplified_Evaluation_Function#Piece-Square_Tables)
//! for more information.
use game::{Board, Color, Piece, Score, Square};

use crate::weights::PIECE_SQUARE_SCORES;

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

/// Evaluates the positional score using piece-square tables.
pub fn evaluate(board: &Board) -> Score {
    evaluate_color(board, Color::White) - evaluate_color(board, Color::Black)
}

fn evaluate_color(board: &Board, color: Color) -> Score {
    let mut score = 0;
    for index in 0..5 {
        let piece = Piece::from(index);
        for square in board.of(piece, color) {
            score += match color {
                Color::White => PIECE_SQUARE_SCORES[piece][MIRRORED[square]],
                Color::Black => PIECE_SQUARE_SCORES[piece][square],
            };
        }
    }
    Score(score)
}
