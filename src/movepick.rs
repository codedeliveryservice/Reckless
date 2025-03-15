use crate::{
    parameters::PIECE_VALUES,
    thread::ThreadData,
    types::{Move, MoveEntry, MoveList},
};

#[derive(PartialEq)]
pub enum Stage {
    HashMove,
    Scoring,
    Others,
}

pub struct MovePicker {
    moves: MoveList,
    tt_move: Move,
    threshold: i32,
    stage: Stage,
}

impl MovePicker {
    pub fn new(td: &ThreadData, tt_move: Move) -> Self {
        Self {
            moves: td.board.generate_all_moves(),
            tt_move,
            threshold: -110,
            stage: Stage::HashMove,
        }
    }

    pub fn new_noisy(td: &ThreadData, include_quiets: bool, threshold: i32) -> Self {
        Self {
            moves: if include_quiets { td.board.generate_all_moves() } else { td.board.generate_capture_moves() },
            tt_move: Move::NULL,
            threshold,
            stage: Stage::Scoring,
        }
    }

    pub fn next(&mut self, td: &ThreadData) -> Option<(Move, i32)> {
        if self.moves.len() == 0 {
            return None;
        }

        if self.stage == Stage::HashMove {
            self.stage = Stage::Scoring;

            for i in 0..self.moves.len() {
                if self.moves[i].mv == self.tt_move {
                    return Some((self.moves.remove(i), 1 << 21));
                }
            }
        }

        if self.stage == Stage::Scoring {
            self.stage = Stage::Others;
            self.score_moves(td);
        }

        let mut index = 0;
        for i in 1..self.moves.len() {
            if self.moves[i].score > self.moves[index].score {
                index = i;
            }
        }

        let score = self.moves[index].score;
        Some((self.moves.remove(index), score))
    }

    fn score_moves(&mut self, td: &ThreadData) {
        for MoveEntry { mv, score } in self.moves.iter_mut() {
            if mv.is_noisy() {
                let captured = td.board.piece_on(mv.to()).piece_type();

                *score = if td.board.see(*mv, self.threshold) { 1 << 20 } else { -(1 << 20) };

                *score += PIECE_VALUES[captured as usize % 6] * 32;

                *score += td.noisy_history.get(&td.board, *mv);
            } else {
                *score = td.quiet_history.get(&td.board, *mv);

                *score += td.conthist(1, *mv);
                *score += td.conthist(2, *mv);
            }
        }
    }
}
