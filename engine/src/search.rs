use game::{
    board::Board,
    core::{Color, Move},
};

use crate::{
    evaluation::{self, score::Score},
    sorting,
};

pub struct SearchResult {
    pub best_move: Move,
    pub score: Score,
    pub nodes: u32,
}

pub fn search(board: &mut Board, depth: u32) -> SearchResult {
    let mut search_engine = InnerSearch::new(board);
    let score = search_engine.perform_search(depth);

    SearchResult {
        best_move: search_engine.best_move,
        nodes: search_engine.nodes,
        score,
    }
}

struct InnerSearch<'a> {
    board: &'a mut Board,
    best_move: Move,
    nodes: u32,
}

impl<'a> InnerSearch<'a> {
    fn new(board: &'a mut Board) -> Self {
        Self {
            board,
            best_move: Move::EMPTY,
            nodes: 0,
        }
    }

    fn perform_search(&mut self, depth: u32) -> Score {
        self.negamax(Score::NEGATIVE_INFINITY, Score::INFINITY, depth)
    }

    /// Implementation of minimax algorithm but instead of using two separate routines for the Min player
    /// and the Max player, it passes on the negated score due to following mathematical relationship:
    ///
    /// `max(a, b) == -min(-a, -b)`
    ///
    /// See [Negamax](https://www.chessprogramming.org/Negamax) for more information.
    fn negamax(&mut self, mut alpha: Score, beta: Score, mut depth: u32) -> Score {
        if depth == 0 {
            return self.quiescence(alpha, beta);
        }

        self.nodes += 1;

        let in_check = self.board.is_in_check();
        // Increase search depth if king is in check
        if in_check {
            depth += 1;
        }

        let mut legal_moves = 0;

        let move_list = self.board.generate_moves();
        let sorted_moves = sorting::sort_moves(self.board, move_list);

        for mv in sorted_moves {
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

        if in_check && legal_moves == 0 {
            // Adding depth eliminates the problem of not choosing the closest path
            // in the case of multiple checkmated positions.
            return Score::CHECKMATE + self.board.depth() as i32;
        } else if legal_moves == 0 {
            return Score::STALEMATE;
        }

        alpha
    }

    /// Quiescence search evaluates only quiet positions, which prevents the horizon effect.
    ///
    /// See [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search)
    /// for more information.
    fn quiescence(&mut self, mut alpha: Score, beta: Score) -> Score {
        self.nodes += 1;

        let evaluation = self.evaluate();

        if evaluation >= beta {
            return beta;
        }

        if alpha < evaluation {
            alpha = evaluation;
        }

        let move_list = self.board.generate_moves();
        let sorted_moves = sorting::sort_moves(self.board, move_list);

        for mv in sorted_moves {
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
