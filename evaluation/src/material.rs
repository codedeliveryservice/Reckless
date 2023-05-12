use game::{Board, Color, Piece, Score};

use crate::weights::MATERIAL_SCORES;

/// Evaluates the material difference between the two players in favor of `Color::White`.
pub fn evaluate(board: &Board) -> Score {
    let mut score = 0;
    for index in 0..5 {
        let piece = Piece::from(index);
        let count = board.of(piece, Color::White).count() - board.of(piece, Color::Black).count();
        score += count as i32 * MATERIAL_SCORES[piece];
    }
    Score(score)
}
