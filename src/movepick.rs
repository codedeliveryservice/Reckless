use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{Move, MoveList},
};

#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub enum Stage {
    HashMove,
    Scoring,
    EverythingElse,
}

pub struct MovePicker {
    list: MoveList,
    tt_move: Move,
    threshold: i32,
    stage: Stage,
}

impl MovePicker {
    pub fn new(td: &ThreadData, tt_move: Move) -> Self {
        Self {
            list: td.board.generate_all_moves(),
            tt_move,
            threshold: -110,
            stage: Stage::HashMove,
        }
    }

    pub fn new_noisy(td: &ThreadData, include_quiets: bool, threshold: i32) -> Self {
        Self {
            list: if include_quiets { td.board.generate_all_moves() } else { td.board.generate_capture_moves() },
            tt_move: Move::NULL,
            threshold,
            stage: Stage::Scoring,
        }
    }

    pub fn next(&mut self, td: &ThreadData) -> Option<(Move, i32)> {
        if self.list.len() == 0 {
            return None;
        }

        if self.stage == Stage::HashMove {
            self.stage = Stage::Scoring;

            let index = self.list.iter().position(|entry| entry.mv == self.tt_move);
            if let Some(index) = index {
                let mv = self.list.remove(index);
                return Some((mv, 1 << 21));
            }
        }

        if self.stage == Stage::Scoring {
            self.stage = Stage::EverythingElse;
            self.score_moves(td);
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

    fn score_moves(&mut self, td: &ThreadData) {
        for entry in self.list.iter_mut() {
            let mv = entry.mv;

            if mv.is_noisy() {
                let captured = td.board.piece_on(mv.to()).piece_type();

                entry.score = if td.board.see(mv, self.threshold) { 1 << 20 } else { -(1 << 20) }
                    + PIECE_VALUES[captured as usize % 6] * 32
                    + td.noisy_history.get(&td.board, mv);
            } else {
                entry.score = td.quiet_history.get(&td.board, mv) + td.conthist(1, mv) + td.conthist(2, mv);
            }
        }
    }
}
