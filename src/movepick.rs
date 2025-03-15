use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{Move, MoveList},
};

pub struct MovePicker {
    list: MoveList,
}

impl MovePicker {
    pub fn new(td: &ThreadData, tt_move: Move) -> Self {
        let mut list = td.board.generate_all_moves();
        score_moves(&mut list, td, tt_move, -110);

        Self { list }
    }

    pub fn new_noisy(td: &ThreadData, include_quiets: bool, threshold: i32) -> Self {
        let mut list = if include_quiets { td.board.generate_all_moves() } else { td.board.generate_capture_moves() };
        score_moves(&mut list, td, Move::NULL, threshold);

        Self { list }
    }

    pub fn next(&mut self) -> Option<(Move, i32)> {
        if self.list.len() == 0 {
            return None;
        }

        let mut index = 0;
        for i in 1..self.list.len() {
            if self.list[i].score > self.list[index].score {
                index = i;
            }
        }

        let entry = self.list[index];
        self.list.remove(index);
        Some((entry.mv, entry.score))
    }
}

#[rustfmt::skip]
fn score_moves(moves: &mut MoveList, td: &ThreadData, tt_move: Move, threshold: i32) {
    for entry in moves.iter_mut() {
        let mv = entry.mv;

        if mv == tt_move {
            entry.score = 1 << 21;
        } else if mv.is_noisy() {
            let captured = td.board.piece_on(mv.to()).piece_type();

            entry.score = if td.board.see(mv, threshold) { 1 << 20 } else { -(1 << 20) }
                + PIECE_VALUES[captured as usize % 6] * 32
                + td.noisy_history.get(&td.board, mv);
        } else {
            entry.score = td.quiet_history.get(&td.board, mv)
                + td.conthist(1, mv)
                + td.conthist(2, mv);
        }
    }
}
