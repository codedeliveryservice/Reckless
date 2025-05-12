use super::{PieceType, MAX_PLY};
use crate::board::Board;

pub struct Score;

#[rustfmt::skip]
impl Score {
    pub const ZERO: i32 = 0;
    pub const DRAW: i32 = 0;

    pub const NONE:     i32 = 32002;
    pub const INFINITE: i32 = 32001;
    pub const MATE:     i32 = 32000;

    pub const MATE_IN_MAX: i32 =  32000 - MAX_PLY as i32;

    pub const TB_WIN:        i32 = Self::MATE_IN_MAX - 1;
    pub const TB_WIN_IN_MAX: i32 = Self::TB_WIN - MAX_PLY as i32;
}

pub const fn mated_in(ply: usize) -> i32 {
    -Score::MATE + ply as i32
}

pub const fn mate_in(ply: usize) -> i32 {
    Score::MATE - ply as i32
}

pub const fn tb_loss_in(ply: usize) -> i32 {
    -Score::TB_WIN + ply as i32
}

pub const fn tb_win_in(ply: usize) -> i32 {
    Score::TB_WIN - ply as i32
}

pub const fn is_win(score: i32) -> bool {
    score >= Score::TB_WIN_IN_MAX
}

pub const fn is_loss(score: i32) -> bool {
    score <= -Score::TB_WIN_IN_MAX
}

pub const fn is_decisive(score: i32) -> bool {
    is_win(score) || is_loss(score)
}

pub const fn is_valid(score: i32) -> bool {
    score != Score::NONE
}

pub fn normalize_to_cp(score: i32, board: &Board) -> i32 {
    let material = board.pieces(PieceType::Pawn).len()
        + 3 * board.pieces(PieceType::Knight).len()
        + 3 * board.pieces(PieceType::Bishop).len()
        + 5 * board.pieces(PieceType::Rook).len()
        + 9 * board.pieces(PieceType::Queen).len();

    let v = material.clamp(16, 64) as f64 / 56.0;

    let normalization = -42.2 * v.powi(3) + 38.0 * v.powi(2) - 13.2 * v + 205.54;

    (100.0 * score as f64 / normalization).round() as i32
}
