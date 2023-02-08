use super::{Move, MoveKind, Square};

/// A data structure similar to `Vec<Move>`, but more efficient and focused solely
/// on collecting and processing `Move` objects.
pub struct MoveList {
    data: [Move; Self::MAX_MOVES],
    index: usize,
}

impl MoveList {
    /// According to the [Chess Programming Wiki](https://www.chessprogramming.org/Encoding_Moves#MoveIndex),
    /// the maximum number of chess moves in a certain position *appears* to be 218.
    /// So make sure the list of moves never gets corrupted.
    const MAX_MOVES: usize = 256;

    /// Creates a new `MoveList`.
    #[inline(always)]
    pub(crate) fn new() -> Self {
        Self {
            data: [Move::EMPTY; Self::MAX_MOVES],
            index: 0,
        }
    }

    /// Creates and appends a new move to the back of the list.
    #[inline(always)]
    pub fn add(&mut self, start: Square, target: Square, move_kind: MoveKind) {
        self.push(Move::new(start, target, move_kind));
    }

    /// Appends a move to the back of the list.
    #[inline(always)]
    pub fn push(&mut self, mv: Move) {
        self.data[self.index] = mv;
        self.index += 1;
    }

    /// Swaps two elements in the list.
    #[inline(always)]
    pub fn swap(&mut self, a: usize, b: usize) {
        self.data.swap(a, b);
    }

    /// Returns the number of moves in the list.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.index
    }
}

impl std::ops::Index<usize> for MoveList {
    type Output = Move;

    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

pub struct MoveListIter {
    list: MoveList,
    index: usize,
}

impl Iterator for MoveListIter {
    type Item = Move;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.list.index {
            let mv = self.list.data[self.index];
            self.index += 1;
            return Some(mv);
        }

        None
    }
}

impl IntoIterator for MoveList {
    type Item = Move;
    type IntoIter = MoveListIter;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        MoveListIter {
            list: self,
            index: 0,
        }
    }
}
