use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{Move, MoveList},
};

enum Kind {
    Normal,
    Noisy,
}

#[derive(Eq, PartialEq, PartialOrd)]
pub enum Stage {
    HashMove,
    Initialization,
    EverythingElse,
}

pub struct MovePicker {
    list: MoveList,
    tt_move: Move,
    killer: Move,
    threshold: i32,
    stage: Stage,
    kind: Kind,
}

impl MovePicker {
    pub const fn new(killer: Move, tt_move: Move) -> Self {
        Self {
            list: MoveList::new(),
            tt_move,
            killer,
            threshold: -110,
            stage: if tt_move.is_valid() { Stage::HashMove } else { Stage::Initialization },
            kind: Kind::Normal,
        }
    }

    pub const fn new_noisy(include_quiets: bool, threshold: i32) -> Self {
        Self {
            list: MoveList::new(),
            tt_move: Move::NULL,
            killer: Move::NULL,
            threshold,
            stage: Stage::Initialization,
            kind: if include_quiets { Kind::Normal } else { Kind::Noisy },
        }
    }

    pub fn next(&mut self, td: &ThreadData) -> Option<(Move, i32)> {
        if self.stage == Stage::HashMove {
            self.stage = Stage::Initialization;

            if td.board.is_pseudo_legal(self.tt_move) {
                return Some((self.tt_move, 1 << 21));
            }
        }

        if self.stage == Stage::Initialization {
            self.stage = Stage::EverythingElse;

            match self.kind {
                Kind::Normal => td.board.append_all_moves(&mut self.list),
                Kind::Noisy => td.board.append_noisy_moves(&mut self.list),
            };

            if let Some(index) = self.list.iter().position(|entry| entry.mv == self.tt_move) {
                self.list.remove(index);
            }

            self.score_moves(td);
        }

        // Stage::EverythingElse
        if self.list.is_empty() {
            return None;
        }

        let mut index = 0;
        for i in 1..self.list.len() {
            if self.list[i].score > self.list[index].score {
                index = i;
            }
        }

        let entry = self.list.remove(index);
        Some((entry.mv, entry.score))
    }

    fn score_moves(&mut self, td: &ThreadData) {
        for entry in self.list.iter_mut() {
            let mv = entry.mv;
            let mut score;

            if mv.is_noisy() {
                let captured = td.board.piece_on(mv.to()).piece_type();

                score = if td.board.see(mv, self.threshold) { 1 << 20 } else { -(1 << 20) };
                score += PIECE_VALUES[captured as usize % 6] * 32;
                score += td.noisy_history.get(&td.board, mv);
            } else {
                score = (1 << 18) * (mv == self.killer) as i32;
                score += td.quiet_history.get(&td.board, mv);
                score += td.conthist(1, mv);
                score += td.conthist(2, mv);
            }

            entry.score = score;
        }
    }
}
