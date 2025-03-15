use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{Move, MoveEntry, MoveList},
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

fn score_moves(moves: &mut MoveList, td: &ThreadData, tt_move: Move, threshold: i32) {
    for MoveEntry { mv, score } in moves.iter_mut() {
        if *mv == tt_move {
            *score = 1 << 21;
            continue;
        }

        if mv.is_noisy() {
            let captured = td.board.piece_on(mv.to()).piece_type();

            *score = if td.board.see(*mv, threshold) { 1 << 20 } else { -(1 << 20) };
            *score += PIECE_VALUES[captured as usize % 6] * 32;
            *score += td.noisy_history.get(&td.board, *mv);
        } else {
            *score = td.quiet_history.get(&td.board, *mv);
            *score += td.conthist(1, *mv);
            *score += td.conthist(2, *mv);
        }
    }
}
