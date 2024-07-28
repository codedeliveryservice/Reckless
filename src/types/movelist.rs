use std::ops::Index;

use super::{Bitboard, Move, MoveKind, Square, MAX_MOVES};

/// A data structure similar to `Vec<Move>`, but more efficient and focused solely
/// on collecting and processing `Move` objects.
pub struct MoveList {
    moves: [Move; MAX_MOVES],
    len: usize,
}

impl MoveList {
    /// Creates a new empty move list.
    pub fn new() -> Self {
        Self { moves: [Move::NULL; MAX_MOVES], len: 0 }
    }

    /// Pushes a move to the end of the list.
    pub fn push(&mut self, mv: Move) {
        self.moves[self.len] = mv;
        self.len += 1;
    }

    /// Creates a new move and adds it to the move list.
    pub fn add(&mut self, start: Square, target: Square, move_kind: MoveKind) {
        self.push(Move::new(start, target, move_kind));
    }

    /// Creates and adds multiple moves to the move list, starting from a common square.
    pub fn add_many(&mut self, start: Square, targets: Bitboard, move_kind: MoveKind) {
        for target in targets {
            self.add(start, target, move_kind);
        }
    }

    /// Retrieves the next move with the highest ordering value from the list.
    pub fn next(&mut self, ordering: &mut [i32]) -> Option<Move> {
        if self.len == 0 {
            return None;
        }

        let mut best = 0;
        for current in 0..self.len {
            if ordering[current] > ordering[best] {
                best = current;
            }
        }

        self.len -= 1;
        ordering.swap(self.len, best);
        self.moves.swap(self.len, best);
        Some(self.moves[self.len])
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }

    pub fn iter(&self) -> std::slice::Iter<Move> {
        self.as_slice().iter()
    }
}

impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}
