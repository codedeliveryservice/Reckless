use std::{ffi, mem, ptr};

use crate::{
    bindings::{
        tb_init, tb_probe_root_dtz, tb_probe_root_wdl, tb_probe_wdl, TbMove, TbRootMove, TbRootMoves, TB_DRAW,
        TB_LARGEST, TB_LOSS, TB_MAX_MOVES, TB_WIN,
    },
    board::Board,
    thread::{RootMove, ThreadData},
    types::{Color, Move, PieceType, Score, MAX_PLY},
};

#[derive(Eq, PartialEq)]
pub enum GameOutcome {
    Win,
    Loss,
    Draw,
}

static mut TB_SIZE: usize = 0;

pub fn tb_initilize(path: &str) -> Option<usize> {
    let cpath = ffi::CString::new(path).ok()?;

    unsafe {
        tb_init(cpath.as_ptr());
        TB_SIZE = TB_LARGEST as usize;
    };

    match tb_size() {
        0 => None,
        _ => Some(tb_size()),
    }
}

pub fn tb_size() -> usize {
    unsafe { TB_SIZE }
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

fn reckless_move_to_tb_move(mv: Move) -> TbMove {
    fn promo_bits_from_piece(pt: PieceType) -> TbMove {
        match pt {
            PieceType::Queen => 1,
            PieceType::Rook => 2,
            PieceType::Bishop => 3,
            PieceType::Knight => 4,
            _ => unreachable!(),
        }
    }

    let from = (mv.from() as u16) & 0x3F;
    let to = (mv.to() as u16) & 0x3F;

    let mut tb_move: TbMove = (from << 6) | to;

    if let Some(pt) = mv.promotion_piece() {
        let promotion_bits = promo_bits_from_piece(pt) & 0x7;
        tb_move |= promotion_bits << 12;
    }

    tb_move
}

pub fn tb_rank_rootmoves(td: &mut ThreadData) {
    let mut rootmoves_in_c: mem::MaybeUninit<TbRootMoves> = mem::MaybeUninit::uninit();

    unsafe {
        let tb_ptr = rootmoves_in_c.as_mut_ptr();

        (*tb_ptr).size = td.root_moves.len().min(TB_MAX_MOVES as usize) as u32;

        for (i, root_move) in td.root_moves.iter().enumerate() {
            if i >= TB_MAX_MOVES as usize {
                break;
            }

            let c_move_ptr = (*tb_ptr).moves.as_mut_ptr().add(i);
            ptr::write(
                c_move_ptr,
                TbRootMove {
                    move_: reckless_move_to_tb_move(root_move.mv),
                    pv: [0; MAX_PLY],
                    pvSize: 0,
                    tbScore: 0,
                    tbRank: 0,
                },
            );
        }

        // Helper to copy back from C struct and sort
        let update_rootmoves = |root_moves: &mut Vec<RootMove>, c_rootmoves: &TbRootMoves| {
            for i in 0..c_rootmoves.size as usize {
                let tb_move = c_rootmoves.moves[i].move_;
                if let Some(rm) = root_moves.iter_mut().find(|rm| reckless_move_to_tb_move(rm.mv) == tb_move) {
                    rm.tb_score = c_rootmoves.moves[i].tbScore;
                    rm.tb_rank = c_rootmoves.moves[i].tbRank;
                }
            }
            root_moves.sort_by(|a, b| b.tb_rank.cmp(&a.tb_rank));
        };

        let dtz_success = tb_probe_root_dtz(
            td.board.colors(Color::White).0,
            td.board.colors(Color::Black).0,
            td.board.pieces(PieceType::King).0,
            td.board.pieces(PieceType::Queen).0,
            td.board.pieces(PieceType::Rook).0,
            td.board.pieces(PieceType::Bishop).0,
            td.board.pieces(PieceType::Knight).0,
            td.board.pieces(PieceType::Pawn).0,
            td.board.halfmove_clock() as u32,
            0,
            td.board.en_passant() as u32 & 0x3F,
            td.board.side_to_move() == Color::White,
            false,
            true,
            tb_ptr,
        );

        if dtz_success != 0 {
            let c_rootmoves: &TbRootMoves = &*tb_ptr;
            update_rootmoves(&mut td.root_moves, c_rootmoves);
            td.root_in_tb = true;
            td.stop_probing_tb = true;
            return;
        }

        // fallback to wdl
        let wdl_success = tb_probe_root_wdl(
            td.board.colors(Color::White).0,
            td.board.colors(Color::Black).0,
            td.board.pieces(PieceType::King).0,
            td.board.pieces(PieceType::Queen).0,
            td.board.pieces(PieceType::Rook).0,
            td.board.pieces(PieceType::Bishop).0,
            td.board.pieces(PieceType::Knight).0,
            td.board.pieces(PieceType::Pawn).0,
            td.board.halfmove_clock() as u32,
            0,
            td.board.en_passant() as u32 & 0x3F,
            td.board.side_to_move() == Color::White,
            true,
            tb_ptr,
        );

        if wdl_success != 0 {
            let c_rootmoves: &TbRootMoves = &*tb_ptr;
            update_rootmoves(&mut td.root_moves, c_rootmoves);
            td.root_in_tb = true;

            // Keep probing in search if DTZ is not available and we are winning
            td.stop_probing_tb = td.root_moves[0].score < Score::DRAW;
        }
    }
}
