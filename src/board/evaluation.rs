use crate::types::{Color, Piece, Square};

include!(concat!(env!("OUT_DIR"), "/psqt.rs"));

#[derive(Debug, Default, Clone, Copy)]
pub struct Evaluation {
    mg: i32,
    eg: i32,
}

impl Evaluation {
    /// Adds the score of a piece at a given square to the current scores using the PSQT.
    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) {
        let (mg, eg) = PSQT[color][piece][square];
        self.mg += mg;
        self.eg += eg;
    }

    /// Subtracts the score of a piece at a given square from the current scores using the PSQT.
    pub fn remove_piece(&mut self, piece: Piece, color: Color, square: Square) {
        let (mg, eg) = PSQT[color][piece][square];
        self.mg -= mg;
        self.eg -= eg;
    }

    /// Returns the current evaluation scores for both middle game and end game phases.
    pub const fn score(self) -> (i32, i32) {
        (self.mg, self.eg)
    }
}
