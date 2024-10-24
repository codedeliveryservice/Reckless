use crate::{
    board::Board,
    parameters::{ordering_counter, ordering_followup, ordering_main},
    tables::History,
    types::{FullMove, Move, MoveList, Piece, MAX_MOVES},
};

#[derive(PartialEq)]
enum Stage {
    HashMove,
    Score,
    Other,
}

pub struct MovePicker {
    stage: Stage,
    moves: MoveList,
    scores: [i32; MAX_MOVES],
    threshold: i32,
    tt_move: Option<Move>,
    killer: Move,
}

impl MovePicker {
    pub fn new(tt_move: Option<Move>, killer: Move, board: &Board) -> Self {
        Self {
            stage: Stage::HashMove,
            moves: board.generate_all_moves(),
            scores: [0; MAX_MOVES],
            threshold: 0,
            tt_move,
            killer,
        }
    }

    pub fn new_noisy(board: &Board) -> Self {
        Self {
            stage: Stage::Score,
            moves: board.generate_capture_moves(),
            scores: [0; MAX_MOVES],
            threshold: 1,
            tt_move: None,
            killer: Move::NULL,
        }
    }

    pub fn next(&mut self, board: &Board, history: &History) -> Option<Move> {
        if self.stage == Stage::HashMove {
            self.stage = Stage::Score;

            if let Some(tt_move) = self.tt_move {
                for (index, &mv) in self.moves.as_slice().iter().enumerate() {
                    if mv == tt_move {
                        self.moves.swap_remove(index);
                        self.scores.swap(index, self.moves.len());

                        return Some(mv);
                    }
                }
            }
        }

        if self.stage == Stage::Score {
            self.stage = Stage::Other;

            let continuations = [1, 2].map(|ply| board.tail_move(ply));
            for (i, mv) in self.moves.as_slice().iter().enumerate() {
                self.scores[i] = self.get_score(*mv, continuations, board, history);
            }
        }

        self.moves.next(&mut self.scores)
    }

    fn get_score(&self, mv: Move, continuations: [FullMove; 2], board: &Board, history: &History) -> i32 {
        if mv.is_capture() {
            let capture = if mv.is_en_passant() { Piece::Pawn } else { board.piece_on(mv.target()) };

            let base = 200_000_000 * if board.see(mv, self.threshold) { 1 } else { -1 };
            let mvv = 1_000_000 * capture as i32;
            let history = history.get_capture(board.side_to_move(), mv, capture);

            return base + mvv + history;
        }

        if mv == self.killer {
            return 100_000_000;
        }

        let piece = board.piece_on(mv.start());

        ordering_main() * history.get_main(board.side_to_move(), mv)
            + ordering_counter() * history.get_counter(continuations[0], piece, mv)
            + ordering_followup() * history.get_followup(continuations[1], piece, mv)
    }
}
