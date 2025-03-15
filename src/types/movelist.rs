use std::ops::Index;

use super::{ArrayVec, Move, MAX_MOVES};

#[derive(Copy, Clone)]
pub struct MoveEntry {
    pub mv: Move,
    pub score: i32,
}

pub struct MoveList {
    data: ArrayVec<MoveEntry, MAX_MOVES>,
}

impl MoveList {
    pub const fn new() -> Self {
        Self { data: ArrayVec::new() }
    }

    pub const fn len(&self) -> usize {
        self.data.len()
    }

    pub fn push(&mut self, item: Move) {
        self.data.push(MoveEntry { mv: item, score: 0 });
    }

    pub fn remove(&mut self, index: usize) -> Move {
        self.data.swap_remove(index).mv
    }

    pub fn iter(&self) -> impl Iterator<Item = &MoveEntry> {
        self.data.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut MoveEntry> {
        self.data.iter_mut()
    }
}

impl Index<usize> for MoveList {
    type Output = MoveEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}
