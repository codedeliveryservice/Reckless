use std::ops::Index;

use super::{ArrayVec, Bitboard, MAX_MOVES, Move, MoveKind, Square};

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct MoveEntry(pub i64);

impl MoveEntry {
    pub const fn new(mv: Move, score: i32) -> MoveEntry {
        MoveEntry(((score as i64) << 32) | mv.0 as i64)
    }

    pub const fn mv(self) -> Move {
        Move(self.0 as u16)
    }

    pub const fn score(self) -> i32 {
        (self.0 >> 32) as i32
    }

    pub fn set_score(&mut self, score: i32) {
        *self = Self::new(self.mv(), score);
    }
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
        self.inner.push(MoveEntry::new(Move::new(from, to, kind), 0));
    }

    #[cfg(not(all(target_feature = "avx512vl", target_feature = "avx512vbmi")))]
    pub fn push_setwise(&mut self, from: Square, to_bb: Bitboard, kind: MoveKind) {
        for to in to_bb {
            self.push(from, to, kind);
        }
    }

    #[cfg(all(target_feature = "avx512vl", target_feature = "avx512vbmi"))]
    pub fn push_setwise(&mut self, from: Square, to_bb: Bitboard, kind: MoveKind) {
        if !to_bb.is_empty() {
            use std::{arch::x86_64::*, mem::transmute};

            unsafe {
                let template0: __m512i = transmute({
                    let mut template0: [Move; 32] = [Move::NULL; 32];
                    for (i, e) in template0.iter_mut().enumerate() {
                        *e = Move::new(Square::new(0u8), Square::new(i as u8), transmute::<u8, MoveKind>(0u8));
                    }
                    template0
                });
                let template1: __m512i = transmute({
                    let mut template1: [Move; 32] = [Move::NULL; 32];
                    for (i, e) in template1.iter_mut().enumerate() {
                        *e = Move::new(Square::new(0u8), Square::new(32 + i as u8), transmute::<u8, MoveKind>(0u8));
                    }
                    template1
                });

                let extra = _mm512_set1_epi16(transmute::<Move, i16>(Move::new(from, Square::new(0u8), kind)));

                self.inner.splat16(to_bb.0 as u32, _mm512_or_si512(template0, extra));
                self.inner.splat16((to_bb.0 >> 32) as u32, _mm512_or_si512(template1, extra));
            }
        }
    }

    #[cfg(not(all(target_feature = "avx512vl", target_feature = "avx512vbmi")))]
    pub fn push_pawns_setwise(&mut self, offset: i8, to_bb: Bitboard, kind: MoveKind) {
        for to in to_bb {
            self.push(to.shift(-offset), to, kind);
        }
    }

    #[cfg(all(target_feature = "avx512vl", target_feature = "avx512vbmi"))]
    pub fn push_pawns_setwise(&mut self, offset: i8, to_bb: Bitboard, kind: MoveKind) {
        if !to_bb.is_empty() {
            use std::{arch::x86_64::*, mem::transmute};

            unsafe {
                let template0: __m512i = transmute({
                    let mut template0: [Move; 32] = [Move::NULL; 32];
                    for (i, e) in template0.iter_mut().enumerate() {
                        let sq = Square::new(i as u8);
                        *e = Move::new(sq, sq, transmute::<u8, MoveKind>(0u8));
                    }
                    template0
                });
                let template1: __m512i = transmute({
                    let mut template1: [Move; 32] = [Move::NULL; 32];
                    for (i, e) in template1.iter_mut().enumerate() {
                        let sq = Square::new(32u8 + i as u8);
                        *e = Move::new(sq, sq, transmute::<u8, MoveKind>(0u8));
                    }
                    template1
                });

                let offset = offset as i16;
                let extra = _mm512_set1_epi16(((kind as i16) << 12).wrapping_sub(offset));

                self.inner.splat8(to_bb.0 as u32, _mm512_add_epi16(template0, extra));
                self.inner.splat8((to_bb.0 >> 32) as u32, _mm512_add_epi16(template1, extra));
            }
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
