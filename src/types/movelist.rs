use std::ops::Index;

use super::{ArrayVec, Bitboard, MAX_MOVES, Move, MoveKind, Square};

#[cfg(target_feature = "avx512vbmi2")]
const _: () = panic!("Disable AVX-512 for now");

#[derive(Copy, Clone)]
#[repr(C)]
pub struct MoveEntry {
    pub mv: Move,
    pub score: i32,
    pub see_value: i32,
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
        self.inner.push(MoveEntry { mv: Move::new(from, to, kind), score: 0, see_value: i32::MIN });
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
        if !to_bb.is_empty() {
            self.push_pawns_setwise(offset, to_bb, MoveKind::PromotionCaptureQ);
            self.push_pawns_setwise(offset, to_bb, MoveKind::PromotionCaptureR);
            self.push_pawns_setwise(offset, to_bb, MoveKind::PromotionCaptureB);
            self.push_pawns_setwise(offset, to_bb, MoveKind::PromotionCaptureN);
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, MoveEntry> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, MoveEntry> {
        self.inner.iter_mut()
    }

    pub const fn remove(&mut self, index: usize) -> MoveEntry {
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
