mod location;
mod material;
mod mobility;

use game::{Board, Color, Score};

/// Returns a statically evaluated `Score` relative to the white side,
/// regardless of the color of the player who is currently making a move.
///
/// Positive values indicate an advantage for white, negative values
/// indicate an advantage for black.
pub fn evaluate_absolute_score(board: &Board) -> Score {
    material::evaluate(board) + location::evaluate(board) + mobility::evaluate(board)
}

/// Returns a statically evaluated `Score` relative to the color
/// of the player who is currently making a move.
pub fn evaluate_relative_score(board: &Board) -> Score {
    match board.turn {
        Color::White => evaluate_absolute_score(board),
        Color::Black => -evaluate_absolute_score(board),
    }
}
