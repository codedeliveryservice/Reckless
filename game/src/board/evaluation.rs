use crate::{Color, Piece, Score, Square};

include!(concat!(env!("OUT_DIR"), "/psqt.rs"));

#[derive(Debug, Default, Clone)]
pub(crate) struct Evaluation {
    pub(crate) score: Score,
}

impl Evaluation {
    #[inline(always)]
    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.score += Score(PSQT[color][piece][square]);
    }

    #[inline(always)]
    pub fn remove_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.score -= Score(PSQT[color][piece][square]);
    }
}
