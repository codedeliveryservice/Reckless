use crate::board::Board;
use crate::types::{Color, Piece};

mod mobility;

pub const DRAW: i32 = 0;
pub const INVALID: i32 = 0;

pub const INFINITY: i32 = 50000;
pub const CHECKMATE: i32 = 48000;
pub const CHECKMATE_BOUND: i32 = 47500;

const MAX_PHASE: i32 = 24;
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

/// Returns a statically evaluated `i32` relative to the white side,
/// regardless of the color of the player who is currently making a move.
///
/// Positive values indicate an advantage for white, negative values
/// indicate an advantage for black.
pub fn evaluate_absolute_score(board: &Board) -> i32 {
    let (mg_psq, eg_psq) = board.psq_score();
    let (mg_mobility, eg_mobility) = mobility::evaluate(board);

    let mg_score = mg_psq + mg_mobility;
    let eg_score = eg_psq + eg_mobility;

    interpolate_score(board, mg_score, eg_score)
}

/// Returns a statically evaluated `i32` relative to the color
/// of the player who is currently making a move.
pub fn evaluate_relative_score(board: &Board) -> i32 {
    match board.turn {
        Color::White => evaluate_absolute_score(board),
        Color::Black => -evaluate_absolute_score(board),
    }
}

pub fn checkmate_in(score: i32) -> Option<i32> {
    if score > CHECKMATE_BOUND {
        return Some((CHECKMATE - score + 1) / 2);
    }
    if score < -CHECKMATE_BOUND {
        return Some((-CHECKMATE - score) / 2);
    }
    None
}

/// Returns a `String` containing a human-readable representation of the evaluation.
pub fn evaluate_debug(board: &Board) -> String {
    let mut result = String::new();

    let (mg_mobility, eg_mobility) = mobility::evaluate(board);
    let (mg_psq, eg_psq) = board.psq_score();

    result.push_str("    TERM    |    MG     EG    TOTAL\n");
    result.push_str("------------|----------------------\n");
    format_score(&mut result, board, "Mobility", mg_mobility, eg_mobility);
    format_score(&mut result, board, "PSQT", mg_psq, eg_psq);
    result.push_str("------------|----------------------\n");

    let mg = mg_mobility + mg_psq;
    let eg = eg_mobility + eg_psq;
    format_score(&mut result, board, "Total", mg, eg);

    let mg = get_phase(board) * 100 / MAX_PHASE;
    let eg = 100 - mg;
    result.push_str(&format!(" Game Phase | {mg:>5}% {eg:>5}%    --\n"));

    result
}

/// Formats the scores and appends them to the result string.
fn format_score(result: &mut String, board: &Board, term: &str, mg: i32, eg: i32) {
    let total = interpolate_score(board, mg, eg) as f64 / 100.0;
    let mg = mg as f64 / 100.0;
    let eg = eg as f64 / 100.0;
    result.push_str(&format!("{term:>11} | {mg:>6.2} {eg:>6.2} {total:>6.2}\n"));
}

/// Interpolates the midgame and endgame scores based on the current phase of the game.
fn interpolate_score(board: &Board, mg_score: i32, eg_score: i32) -> i32 {
    let phase = get_phase(board);
    (mg_score * phase + eg_score * (MAX_PHASE - phase)) / MAX_PHASE
}

/// Calculates the current phase of the game based on the number of pieces on the board.
///
/// The phase is a number between `0` and `24`, where `24` means the game is in the opening
/// and `0` means the game is in the endgame.
fn get_phase(board: &Board) -> i32 {
    [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen]
        .iter()
        .map(|&piece| board.pieces(piece).count() as i32 * PHASE_WEIGHTS[piece])
        .sum::<i32>()
        .min(MAX_PHASE) // In case of early promotions
}
