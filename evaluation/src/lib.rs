mod location;
mod material;
mod mobility;

use game::{Board, Score};

/// Returns a statically evaluated `Score` relative to the white side,
/// which means that positive values are better for the white side.
pub fn evaluate(board: &Board) -> Score {
    material::evaluate(board) + location::evaluate(board) + mobility::evaluate(board)
}
