mod material;
mod mobility;

use game::{Board, Color, Piece, Score};

const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
const TOTAL_PHASE: i32 = 24;

/// Returns a statically evaluated `Score` relative to the white side,
/// regardless of the color of the player who is currently making a move.
///
/// Positive values indicate an advantage for white, negative values
/// indicate an advantage for black.
pub fn evaluate_absolute_score(board: &Board) -> Score {
    let (mg_psq, eg_psq) = board.psq_score();
    let (mg_material, eg_material) = material::evaluate(board);
    let (mg_mobility, eg_mobility) = mobility::evaluate(board);

    let mg_score = mg_psq + mg_material + mg_mobility;
    let eg_score = eg_psq + eg_material + eg_mobility;

    let mg_phase = get_phase(board);
    let eg_phase = TOTAL_PHASE - mg_phase;

    (mg_score * mg_phase + eg_score * eg_phase) / TOTAL_PHASE
}

/// Returns a statically evaluated `Score` relative to the color
/// of the player who is currently making a move.
pub fn evaluate_relative_score(board: &Board) -> Score {
    match board.turn {
        Color::White => evaluate_absolute_score(board),
        Color::Black => -evaluate_absolute_score(board),
    }
}

/// Calculates the current phase of the game based on the number of pieces on the board.
///
/// The phase is a number between 0 and 24, where 0 means the game is in the opening
/// and 24 means the game is in the endgame.
fn get_phase(board: &Board) -> i32 {
    [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen]
        .iter()
        .map(|&piece| board.pieces(piece).count() as i32 * PHASE_WEIGHTS[piece])
        .sum::<i32>()
        .min(TOTAL_PHASE) // In case of early promotions
}
