use std::ffi::CString;

use crate::{
    bindings::{tb_init, tb_probe_wdl, TB_DRAW, TB_LARGEST, TB_LOSS, TB_WIN},
    board::Board,
    types::{Color, PieceType},
};

#[derive(PartialEq)]
pub enum GameOutcome {
    Win,
    Loss,
    Draw,
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

pub fn tb_probe(board: &Board) -> Option<GameOutcome> {
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
            board.en_passant() as u32 & 0x3F,
            board.side_to_move() == Color::White,
        )
    };

    match code {
        TB_WIN => Some(GameOutcome::Win),
        TB_LOSS => Some(GameOutcome::Loss),
        TB_DRAW => Some(GameOutcome::Draw),
        _ => None,
    }
}
