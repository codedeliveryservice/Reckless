use crate::types::{Move, MAX_PLY};

/// Triangular principle variation table.
///
/// See [Triangular PV Table](https://www.chessprogramming.org/Triangular_PV-Table) for more information.
pub struct PrincipleVariationTable {
    table: [[Move; MAX_PLY + 1]; MAX_PLY + 1],
    length: [usize; MAX_PLY + 1],
}

impl PrincipleVariationTable {
    /// Returns the principle variation line of the current search.
    ///
    /// This method should only be called after a search has been performed.
    pub fn get_line(&self) -> Vec<Move> {
        let mut line = Vec::with_capacity(self.length[0]);
        for i in 0..self.length[0] {
            line.push(self.table[0][i]);
        }
        line
    }

    pub fn clear(&mut self, ply: usize) {
        self.length[ply] = 0;
    }

    pub fn update(&mut self, ply: usize, mv: Move) {
        self.table[ply][0] = mv;
        self.length[ply] = self.length[ply + 1] + 1;
        for i in 0..self.length[ply + 1] {
            self.table[ply][i + 1] = self.table[ply + 1][i];
        }
    }
}

impl Default for PrincipleVariationTable {
    fn default() -> Self {
        Self {
            table: [[Move::NULL; MAX_PLY + 1]; MAX_PLY + 1],
            length: [0; MAX_PLY + 1],
        }
    }
}
