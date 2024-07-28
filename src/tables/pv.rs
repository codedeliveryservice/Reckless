use crate::types::{Move, MAX_PLY};

/// Implementation of a [Triangular Principal Variation Table][wiki].
///
/// [wiki]: https://www.chessprogramming.org/Triangular_PV-Table
pub struct PrincipalVariationTable {
    table: [[Move; MAX_PLY + 1]; MAX_PLY + 1],
    len: [usize; MAX_PLY + 1],
}

impl PrincipalVariationTable {
    pub fn best_move(&self) -> Move {
        self.table[0][0]
    }

    pub fn variation(&self) -> Vec<Move> {
        self.table[0][..self.len[0]].to_vec()
    }

    pub fn clear(&mut self, ply: usize) {
        self.len[ply] = 0;
    }

    pub fn update(&mut self, ply: usize, mv: Move) {
        self.table[ply][0] = mv;
        self.len[ply] = self.len[ply + 1] + 1;
        for i in 0..self.len[ply + 1] {
            self.table[ply][i + 1] = self.table[ply + 1][i];
        }
    }
}

impl Default for PrincipalVariationTable {
    fn default() -> Self {
        Self {
            table: [[Move::NULL; MAX_PLY + 1]; MAX_PLY + 1],
            len: [0; MAX_PLY + 1],
        }
    }
}
