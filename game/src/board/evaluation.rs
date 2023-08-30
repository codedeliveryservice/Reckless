use crate::{Color, Piece, Score, Square};

include!(concat!(env!("OUT_DIR"), "/psqt.rs"));

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Evaluation {
    mg: Score,
    eg: Score,
}

impl Evaluation {
    /// Adds the score of a piece at a given square to the current scores using the PSQT.
    #[inline(always)]
    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) {
        let (mg, eg) = PSQT[color][piece][square];
        self.mg += Score(mg);
        self.eg += Score(eg);
    }

    /// Subtracts the score of a piece at a given square from the current scores using the PSQT.
    #[inline(always)]
    pub fn remove_piece(&mut self, piece: Piece, color: Color, square: Square) {
        let (mg, eg) = PSQT[color][piece][square];
        self.mg -= Score(mg);
        self.eg -= Score(eg);
    }

    /// Returns the current evaluation scores for both middle game and end game phases.
    pub fn score(&self) -> (Score, Score) {
        (self.mg, self.eg)
    }
}
