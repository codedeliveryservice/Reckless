use std::ops::Index;

use super::{Bitboard, Move, MoveKind, Square, MAX_MOVES};

/// A data structure similar to `Vec<Move>`, but more efficient and focused solely
/// on collecting and processing `Move` objects.
pub struct MoveList {
    moves: [Move; MAX_MOVES],
    length: usize,
}

impl MoveList {
    /// Creates a new empty move list.
    pub fn new() -> Self {
        Self {
            moves: [Move::NULL; MAX_MOVES],
            length: 0,
        }
    }

    /// Pushes a move to the end of the list.
    pub fn push(&mut self, mv: Move) {
        self.moves[self.length] = mv;
        self.length += 1;
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
        if self.length == 0 {
            return None;
        }

        let mut best = 0;
        for current in 0..self.length {
            if ordering[current] > ordering[best] {
                best = current;
            }
        }

        self.length -= 1;
        ordering.swap(self.length, best);
        self.moves.swap(self.length, best);
        Some(self.moves[self.length])
    }

    /// Returns the number of moves in the list.
    pub const fn length(&self) -> usize {
        self.length
    }

    /// Returns an iterator over the list of moves.
    pub const fn iter(&self) -> MoveListIter {
        MoveListIter { list: self, index: 0 }
    }
}

impl Index<usize> for MoveList {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}

pub struct MoveListIter<'a> {
    list: &'a MoveList,
    index: usize,
}

impl<'a> Iterator for MoveListIter<'a> {
    type Item = Move;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.length {
            let mv = self.list.moves[self.index];
            self.index += 1;
            return Some(mv);
        }
        None
    }
}
