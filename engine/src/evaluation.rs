mod material;
pub mod score;

use game::board::Board;

use self::score::Score;

pub fn evaluate(board: &Board) -> Score {
    material::evaluate_material(board)
}
