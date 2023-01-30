mod material;

pub mod score;

use game::board::Board;

use self::score::Score;

/// Returns a statically evaluated `Score` relative to the white side,
/// which means that positive values are better for the white side.
pub fn evaluate(board: &Board) -> Score {
    material::evaluate_material(board)
}
