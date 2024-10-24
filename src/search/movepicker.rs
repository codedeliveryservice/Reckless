use crate::{
    board::Board,
    parameters::{ordering_counter, ordering_followup, ordering_main},
    tables::History,
    types::{FullMove, Move, MoveList, Piece, MAX_MOVES},
};

pub struct MovePicker {
    moves: MoveList,
    scores: [i32; MAX_MOVES],
}

impl MovePicker {
    pub fn new(tt_move: Option<Move>, killer: Move, board: &Board, history: &History) -> Self {
        let continuations = [1, 2].map(|ply| board.tail_move(ply));
        let moves = board.generate_all_moves();

        let mut scores = [0; MAX_MOVES];
        for (score, mv) in scores.iter_mut().zip(moves.as_slice()) {
            *score = get_score(*mv, tt_move, killer, 0, continuations, board, history);
        }

        Self { moves, scores }
    }

    pub fn new_noisy(board: &Board, history: &History) -> Self {
        let moves = board.generate_capture_moves();

        let mut scores = [0; MAX_MOVES];
        for (score, mv) in scores.iter_mut().zip(moves.as_slice()) {
            *score = get_score(*mv, None, Move::NULL, 1, [FullMove::NULL; 2], board, history);
        }

        Self { moves, scores }
    }

    pub fn next(&mut self) -> Option<Move> {
        self.moves.next(&mut self.scores)
    }
}

fn get_score(
    mv: Move,
    tt_move: Option<Move>,
    killer: Move,
    threshold: i32,
    continuations: [FullMove; 2],
    board: &Board,
    history: &History,
) -> i32 {
    if Some(mv) == tt_move {
        return 300_000_000;
    }

    if mv.is_capture() {
        let capture = if mv.is_en_passant() { Piece::Pawn } else { board.piece_on(mv.target()) };

        let base = 200_000_000 * if board.see(mv, threshold) { 1 } else { -1 };
        let mvv = 1_000_000 * capture as i32;
        let history = history.get_capture(board.side_to_move(), mv, capture);

        return base + mvv + history;
    }

    if mv == killer {
        return 100_000_000;
    }

    let piece = board.piece_on(mv.start());

    ordering_main() * history.get_main(board.side_to_move(), mv)
        + ordering_counter() * history.get_counter(continuations[0], piece, mv)
        + ordering_followup() * history.get_followup(continuations[1], piece, mv)
}
