mod material;
mod mobility;

use game::{Board, Color, Piece, Score};

/// The phase weights for each piece type.
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];
/// The maximum reachable phase value (all pieces on the board).
const MAX_PHASE: i32 = 24;

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

    interpolate_score(board, mg_score, eg_score)
}

/// Returns a statically evaluated `Score` relative to the color
/// of the player who is currently making a move.
pub fn evaluate_relative_score(board: &Board) -> Score {
    match board.turn {
        Color::White => evaluate_absolute_score(board),
        Color::Black => -evaluate_absolute_score(board),
    }
}

/// Returns a `String` containing a human-readable representation of the evaluation.
///
/// # Examples
///
/// ```
/// use game::Board;
/// use evaluation::evaluate_debug;
///
/// let board = Board::new("b3rrk1/3q1ppp/p1N3n1/1pRP4/3N4/8/PP3PPP/3Q1RK1 b - - 8 23").unwrap();
/// println!("{}", evaluate_debug(&board));
/// ```
///
/// Output:
///
/// ```plaintext
///     TERM    |    MG     EG    TOTAL
/// ------------|----------------------
///    Material |   0.51   1.36   0.65
///    Mobility |   0.25   1.66   0.48
///        PSQT |   0.44   0.75   0.49
/// ------------|----------------------
///       Total |   1.20   3.77   1.62
///  Game Phase |    83%    17%    --
/// ```
pub fn evaluate_debug(board: &Board) -> String {
    let mut result = String::new();

    let (mg_material, eg_material) = material::evaluate(board);
    let (mg_mobility, eg_mobility) = mobility::evaluate(board);
    let (mg_psq, eg_psq) = board.psq_score();

    result.push_str(&format!("    TERM    |    MG     EG    TOTAL\n",));
    result.push_str(&format!("------------|----------------------\n",));
    format_score(&mut result, board, "Material", mg_material, eg_material);
    format_score(&mut result, board, "Mobility", mg_mobility, eg_mobility);
    format_score(&mut result, board, "PSQT", mg_psq, eg_psq);
    result.push_str(&format!("------------|----------------------\n",));

    let mg = mg_material + mg_mobility + mg_psq;
    let eg = eg_material + eg_mobility + eg_psq;
    format_score(&mut result, board, "Total", mg, eg);

    let mg = get_phase(board) * 100 / MAX_PHASE;
    let eg = 100 - mg;
    result.push_str(&format!(" Game Phase | {:>5}% {:>5}%    --\n", mg, eg));

    result
}

/// Formats the scores and appends them to the result string.
fn format_score(result: &mut String, board: &Board, term: &str, mg: Score, eg: Score) {
    result.push_str(&format!(
        "{:>11} | {:>6.2} {:>6.2} {:>6.2}\n",
        term,
        mg.0 as f64 / 100.0,
        eg.0 as f64 / 100.0,
        interpolate_score(board, mg, eg).0 as f64 / 100.0,
    ));
}

/// Interpolates the midgame and endgame scores based on the current phase of the game.
fn interpolate_score(board: &Board, mg_score: Score, eg_score: Score) -> Score {
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
