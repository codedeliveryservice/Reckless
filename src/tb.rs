use std::ffi::CString;

use crate::{
    bindings::{tb_init, tb_probe_wdl, TB_BLESSED_LOSS, TB_CURSED_WIN, TB_DRAW, TB_LARGEST, TB_LOSS, TB_WIN},
    board::Board,
    types::{Color, PieceType},
};

pub enum TbResult {
    Win,
    Loss,
    Draw,
    BlessedLoss,
    CursedWin,
    Failed,
}

pub fn tb_initilize(path: &str) -> Option<usize> {
    let cpath = CString::new(path).ok()?;

    unsafe { tb_init(cpath.as_ptr()) };

    match tb_size() {
        0 => None,
        _ => Some(tb_size()),
    }
}

pub fn tb_size() -> usize {
    unsafe { TB_LARGEST as usize }
}

pub fn tb_probe(board: &Board) -> TbResult {
    let code = unsafe {
        tb_probe_wdl(
            board.colors(Color::White).0,
            board.colors(Color::Black).0,
            board.pieces(PieceType::King).0,
            board.pieces(PieceType::Queen).0,
            board.pieces(PieceType::Rook).0,
            board.pieces(PieceType::Bishop).0,
            board.pieces(PieceType::Knight).0,
            board.pieces(PieceType::Pawn).0,
            0,
            0,
            0,
            board.side_to_move() == Color::White,
        )
    };

    match code {
        TB_WIN => TbResult::Win,
        TB_LOSS => TbResult::Loss,
        TB_DRAW => TbResult::Draw,
        TB_BLESSED_LOSS => TbResult::BlessedLoss,
        TB_CURSED_WIN => TbResult::CursedWin,
        _ => TbResult::Failed,
    }
}
