mod killer_moves;
mod mvv_lva;

use std::time::Instant;

use self::killer_moves::KillerMoves;

use crate::{
    evaluation,
    uci::{self, UciMessage},
};

use game::{Board, Color, Move, MoveList, Score};

pub fn search(board: &mut Board, depth: u32) {
    InnerSearch::new(board).perform_search(depth);
}

struct InnerSearch<'a> {
    board: &'a mut Board,
    nodes: u32,
    ply: usize,
    killers: KillerMoves<2>,
}

impl<'a> InnerSearch<'a> {
    fn new(board: &'a mut Board) -> Self {
        Self {
            board,
            nodes: Default::default(),
            ply: Default::default(),
            killers: KillerMoves::new(),
        }
    }

    fn perform_search(&mut self, max_depth: u32) {
        for depth in 1..=max_depth {
            self.nodes = 0;

            let mut pv = vec![];

            let now = Instant::now();
            let score = self.negamax(Score::NEGATIVE_INFINITY, Score::INFINITY, depth, &mut pv);
            let duration = now.elapsed();

            uci::send(UciMessage::SearchReport {
                depth,
                score,
                duration,
                pv: pv.to_vec(),
                nodes: self.nodes,
            });

            if depth == max_depth {
                uci::send(UciMessage::BestMove(pv[0]));
            }
        }
    }

    /// Implementation of minimax algorithm but instead of using two separate routines for the Min player
    /// and the Max player, it passes on the negated score due to following mathematical relationship:
    ///
    /// `max(a, b) == -min(-a, -b)`
    ///
    /// See [Negamax](https://www.chessprogramming.org/Negamax) for more information.
    fn negamax(
        &mut self,
        mut alpha: Score,
        beta: Score,
        mut depth: u32,
        pv: &mut Vec<Move>,
    ) -> Score {
        if self.ply > 0 && self.board.is_repetition() {
            return Score::ZERO;
        }

        if depth == 0 {
            return self.quiescence(alpha, beta);
        }

        self.nodes += 1;

        // Increase search depth if king is in check
        let in_check = self.board.is_in_check();
        if in_check {
            depth += 1;
        }

        let mut legal_moves = 0;

        let moves = self.sort_moves(self.board.generate_moves());
        for mv in moves {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            legal_moves += 1;
            self.ply += 1;

            let mut child_pv = vec![];
            let score = -self.negamax(-beta, -alpha, depth - 1, &mut child_pv);

            self.board.take_back();
            self.ply -= 1;

            // Perform a fail-hard beta cutoff
            if score >= beta {
                // The killer heuristic is intended only for ordering quiet moves
                if mv.is_quiet() {
                    self.killers.add(mv, self.ply);
                }

                return beta;
            }

            // Found a better move that maximizes alpha
            if alpha < score {
                alpha = score;

                pv.clear();
                pv.push(mv);
                pv.extend(&child_pv);
            }
        }

        if legal_moves == 0 {
            return match in_check {
                // Since negamax evaluates positions from the point of view of the maximizing player,
                // we choose the longest path to checkmate by adding the depth (maximizing the score)
                true => Score::CHECKMATE + self.ply as i32,
                false => Score::STALEMATE,
            };
        }

        alpha
    }

    /// Quiescence search evaluates only quiet positions, which prevents the horizon effect.
    ///
    /// See [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search)
    /// for more information.
    fn quiescence(&mut self, mut alpha: Score, beta: Score) -> Score {
        self.nodes += 1;

        // Negamax requires the static evaluation function to return a score relative to the side being evaluated
        let evaluation = match self.board.turn {
            Color::White => evaluation::evaluate(self.board),
            Color::Black => -evaluation::evaluate(self.board),
        };

        if evaluation >= beta {
            return beta;
        }

        if alpha < evaluation {
            alpha = evaluation;
        }

        let moves = self.sort_moves(self.board.generate_moves());
        for mv in moves {
            if !mv.is_capture() || self.board.make_move(mv).is_err() {
                continue;
            }

            let score = -self.quiescence(-beta, -alpha);
            self.board.take_back();

            if score >= beta {
                return beta;
            }

            if alpha < score {
                alpha = score;
            }
        }

        alpha
    }

    fn sort_moves(&self, mut moves: MoveList) -> MoveList {
        let mut scores = vec![0; moves.len()];
        for index in 0..moves.len() {
            scores[index] = self.score_move(moves[index]);
        }

        for current in 0..moves.len() {
            for compared in (current + 1)..moves.len() {
                if scores[current] < scores[compared] {
                    scores.swap(current, compared);
                    moves.swap(current, compared);
                }
            }
        }

        moves
    }

    /// Returns a move score based on heuristic analysis.
    fn score_move(&self, mv: Move) -> u32 {
        if mv.is_capture() {
            return mvv_lva::score_mvv_lva(self.board, mv);
        }

        if self.killers.contains(mv, self.ply) {
            // The quiet move score is rated below any capture move
            return 90;
        }

        Default::default()
    }
}
