use crate::board::Board;
use crate::types::{Bitboard, Color, Piece, Square, S};

const MAX_PHASE: i32 = 24;
const PHASE_WEIGHTS: [i32; 6] = [0, 1, 1, 2, 4, 0];

const TEMPO_BONUS: i32 = 15;

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
        let our_king = board.king(color);
        let their_king = board.king(!color);

        for piece in 0..5 {
            let piece = Piece::from(piece);

            for square in board.of(piece, color) {
                score += WEIGHTS.psqt[0][our_king ^ flip][piece][square ^ flip];
                score += WEIGHTS.psqt[1][their_king ^ flip][piece][square ^ flip];

                score += match piece {
                    Piece::Pawn if is_passed_pawn(board, square, color) => WEIGHTS.passed_pawns[square ^ flip],
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

/// Returns `true` if the pawn has no opposing pawns in front on the same or adjacent files.
fn is_passed_pawn(board: &Board, square: Square, color: Color) -> bool {
    (passed_pawn_mask(square, color) & board.of(Piece::Pawn, !color)).is_empty()
}

/// Returns a `Bitboard` with the squares in front of the square on the same and adjacent files.
const fn passed_pawn_mask(square: Square, color: Color) -> Bitboard {
    let mut mask = match color {
        Color::White => 0x0101010101010100 << square.0,
        Color::Black => 0x0080808080808080 >> (63 - square.0),
    };
    mask |= !0x0101010101010101 & (mask << 1);
    mask |= !0x8080808080808080 & (mask >> 1);
    Bitboard(mask)
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
    /// Piece-square tables based on the positions of kings `[<our/their>][king][piece][square]`
    psqt: [[[[S; Square::NUM]; Piece::NUM - 1]; Square::NUM]; 2],
    bishop_mobility: [S; 14],
    rook_mobility: [S; 15],
    queen_mobility: [S; 28],
    passed_pawns: [S; Square::NUM],
}

static WEIGHTS: Weights = unsafe { std::mem::transmute(*include_bytes!("../data/weights.bin")) };
