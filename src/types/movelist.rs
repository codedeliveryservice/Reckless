use std::ops::Index;

use super::{ArrayVec, Bitboard, Move, MoveKind, Square, MAX_MOVES};

#[derive(Copy, Clone)]
pub struct MoveEntry {
    pub mv: Move,
    pub score: i32,
}

pub struct MoveList {
    inner: ArrayVec<MoveEntry, MAX_MOVES>,
}

impl MoveList {
    pub const fn new() -> Self {
        Self { inner: ArrayVec::new() }
    }

    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn push(&mut self, from: Square, to: Square, kind: MoveKind) {
        self.inner.push(MoveEntry { mv: Move::new(from, to, kind), score: 0 });
    }

    pub fn push_setwise(&mut self, from: Square, to_bb: Bitboard, kind: MoveKind) {
        for to in to_bb {
            self.push(from, to, kind);
        }
    }

    pub fn push_pawns_setwise(&mut self, offset: i8, to_bb: Bitboard, kind: MoveKind) {
        for to in to_bb {
            self.push(to.shift(-offset), to, kind);
        }
    }

    pub fn push_promotion_capture_setwise(&mut self, offset: i8, to_bb: Bitboard) {
        for to in to_bb {
            let from = to.shift(-offset);
            self.push(from, to, MoveKind::PromotionCaptureQ);
            self.push(from, to, MoveKind::PromotionCaptureR);
            self.push(from, to, MoveKind::PromotionCaptureB);
            self.push(from, to, MoveKind::PromotionCaptureN);
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MoveEntry> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, MoveEntry> {
        self.inner.iter_mut()
    }

    pub fn remove(&mut self, index: usize) -> MoveEntry {
        self.inner.swap_remove(index)
    }
}

impl Index<usize> for MoveList {
    type Output = MoveEntry;

    fn index(&self, index: usize) -> &Self::Output {
        self.inner.get(index)
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}
