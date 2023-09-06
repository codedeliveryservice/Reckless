use super::{Move, MoveKind, Square};

/// A data structure similar to `Vec<Move>`, but more efficient and focused solely
/// on collecting and processing `Move` objects.
pub struct MoveList {
    moves: [Move; Self::MAX_MOVES],
    length: usize,
}

impl MoveList {
    /// According to [Chess Programming Wiki](https://www.chessprogramming.org/Encoding_Moves#MoveIndex),
    /// the maximum number of chess moves in a certain position *appears* to be 218.
    /// So make sure the list of moves never gets corrupted.
    const MAX_MOVES: usize = 256;

    /// Creates a new `MoveList`.
    pub(crate) fn new() -> Self {
        Self {
            moves: [Move::default(); Self::MAX_MOVES],
            length: 0,
        }
    }

    /// Creates and appends a new move to the back of the list.
    pub fn add(&mut self, start: Square, target: Square, move_kind: MoveKind) {
        self.moves[self.length] = Move::new(start, target, move_kind);
        self.length += 1;
    }

    pub fn next(&mut self, ordering: &mut [u32]) -> Option<Move> {
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

impl std::ops::Index<usize> for MoveList {
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
