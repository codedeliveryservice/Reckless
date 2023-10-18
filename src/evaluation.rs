use crate::board::Board;
use crate::types::{Color, Piece, Square, S};

pub const DRAW: i32 = 0;
pub const INVALID: i32 = 0;

pub const INFINITY: i32 = 50000;
pub const CHECKMATE: i32 = 48000;
pub const CHECKMATE_BOUND: i32 = 47500;

const MAX_PHASE: i32 = 24;
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

const TEMPO_BONUS: i32 = 15;

/// Returns the number of full moves to checkmate.
pub fn checkmate_in(score: i32) -> Option<i32> {
    if score > CHECKMATE_BOUND {
        return Some((CHECKMATE - score + 1) / 2);
    }
    if score < -CHECKMATE_BOUND {
        return Some((-CHECKMATE - score) / 2);
    }
    None
}

/// Returns a statically evaluated score relative to the color
/// of the player who is currently making a move.
pub fn evaluate(board: &Board) -> i32 {
    let (mg_score, eg_score) = evaluate_internal(board).deconstruct();

    let phase = get_phase(board);
    let score = (mg_score * phase + eg_score * (MAX_PHASE - phase)) / MAX_PHASE;

    match board.turn {
        Color::White => score + TEMPO_BONUS,
        Color::Black => -score + TEMPO_BONUS,
    }
}

fn evaluate_internal(board: &Board) -> S {
    let mut score = S::default();
    for (color, flip) in [Color::White, Color::Black].into_iter().zip([0, 56]) {
        let king = board.king(color);

        for piece in 0..5 {
            let piece = Piece::from(piece);

            for square in board.of(piece, color) {
                score += WEIGHTS.psqt[king ^ flip][piece][square ^ flip];

                score += match piece {
                    Piece::Bishop => WEIGHTS.bishop_mobility[board.get_attacks(square, piece).count() as usize],
                    Piece::Rook => WEIGHTS.rook_mobility[board.get_attacks(square, piece).count() as usize],
                    Piece::Queen => WEIGHTS.queen_mobility[board.get_attacks(square, piece).count() as usize],
                    _ => continue,
                }
            }
        }

        score = -score;
    }
    score
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

#[repr(C)]
struct Weights {
    /// Piece-square tables based on the current king position `[king][piece][square]`
    psqt: [[[S; Square::NUM]; Piece::NUM - 1]; Square::NUM],
    bishop_mobility: [S; 14],
    rook_mobility: [S; 15],
    queen_mobility: [S; 28],
}

static WEIGHTS: Weights = unsafe { std::mem::transmute(*include_bytes!("../data/weights.bin")) };
