use game::{
    board::Board,
    core::{Color, Move},
};

use crate::evaluation::{self, score::Score};

pub fn search(board: &mut Board, depth: u32) -> Move {
    let mut search_engine = InnerSearch::new(board);
    search_engine.perform_search(depth);
    search_engine.best_move
}

struct InnerSearch<'a> {
    board: &'a mut Board,
    best_move: Move,
}

impl<'a> InnerSearch<'a> {
    const INFINITY: Score = Score(50000);
    const CHECKMATE: Score = Score(48000);

    fn new(board: &'a mut Board) -> Self {
        Self {
            board,
            best_move: Move::EMPTY,
        }
    }

    fn perform_search(&mut self, depth: u32) {
        self.negamax(-Self::INFINITY, Self::INFINITY, depth);
    }

    /// Implementation of minimax algorithm but instead of using two separate routines for the Min player
    /// and the Max player, it passes on the negated score due to following mathematical relationship:
    ///
    /// `max(a, b) == -min(-a, -b)`
    ///
    /// See [Negamax](https://www.chessprogramming.org/Negamax) for more information.
    fn negamax(&mut self, mut alpha: Score, beta: Score, depth: u32) -> Score {
        if depth == 0 {
            return self.quiescence(alpha, beta);
        }

        let mut legal_moves = 0;

        for mv in self.board.generate_moves() {
            if self.board.make_move(mv).is_err() {
                continue;
            }

            legal_moves += 1;
            let score = -self.negamax(-beta, -alpha, depth - 1);
            self.board.take_back();

            if score >= beta {
                return beta;
            }

            if alpha < score {
                alpha = score;

                let root_node = self.board.depth() == 0;
                if root_node {
                    self.best_move = mv;
                }
            }
        }

        if legal_moves == 0 {
            // TODO: Add check for stalemate
            return self.checkmate_score();
        }

        alpha
    }

    fn checkmate_score(&mut self) -> Score {
        // Adding depth eliminates the problem of not choosing the closest path
        // in the case of multiple checkmated positions.
        -Self::CHECKMATE + self.board.depth() as i32
    }

    /// Quiescence search evaluates only quiet positions, which prevents the horizon effect.
    ///
    /// See [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search)
    /// for more information.
    fn quiescence(&mut self, mut alpha: Score, beta: Score) -> Score {
        let evaluation = self.evaluate();

        if evaluation >= beta {
            return beta;
        }

        if alpha < evaluation {
            alpha = evaluation;
        }

        for mv in self.board.generate_moves() {
            if !mv.is_capture() {
                continue;
            }

            if self.board.make_move(mv).is_err() {
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

    fn evaluate(&mut self) -> Score {
        // Negamax requires the static evaluation function to return a score relative to the side being evaluated
        match self.board.turn {
            Color::White => evaluation::evaluate(self.board),
            Color::Black => -evaluation::evaluate(self.board),
        }
    }
}
