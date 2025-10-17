use std::{
    io::Write,
    sync::{Arc, Mutex, OnceLock},
};

use crate::{
    board::Board,
    types::{Color, Move, PieceType, MAX_PLY},
};

use accumulator::{Accumulator, AccumulatorCache};
use libc::{c_uint, pthread_self, pthread_setaffinity_np, syscall, SYS_getcpu, CPU_SET, CPU_ZERO};
use memmap2::Mmap;
use tempfile::NamedTempFile;

mod accumulator;

mod forward {
    #[cfg(target_feature = "avx2")]
    mod vectorized;
    #[cfg(target_feature = "avx2")]
    pub use vectorized::*;

    #[cfg(not(target_feature = "avx2"))]
    mod scalar;
    #[cfg(not(target_feature = "avx2"))]
    pub use scalar::*;
}

mod simd {
    #[cfg(target_feature = "avx512f")]
    mod avx512;
    #[cfg(target_feature = "avx512f")]
    pub use avx512::*;

    #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
    mod avx2;
    #[cfg(all(target_feature = "avx2", not(target_feature = "avx512f")))]
    pub use avx2::*;

    #[cfg(all(not(target_feature = "avx2"), not(target_feature = "avx512f")))]
    mod scalar;
    #[cfg(all(not(target_feature = "avx2"), not(target_feature = "avx512f")))]
    pub use scalar::*;
}

const NETWORK_SCALE: i32 = 400;

const INPUT_BUCKETS: usize = 10;

const FT_SIZE: usize = 768;
const L1_SIZE: usize = 1024;
const L2_SIZE: usize = 16;
const L3_SIZE: usize = 32;

const FT_QUANT: i32 = 255;
const L1_QUANT: i32 = 64;

#[cfg(target_feature = "avx512f")]
const FT_SHIFT: u32 = 9;
#[cfg(not(target_feature = "avx512f"))]
const FT_SHIFT: i32 = 9;

const DEQUANT_MULTIPLIER: f32 = (1 << FT_SHIFT) as f32 / (FT_QUANT * FT_QUANT * L1_QUANT) as f32;

#[rustfmt::skip]
const BUCKETS: [usize; 64] = [
    0, 1, 2, 3, 3, 2, 1, 0,
    4, 5, 6, 7, 7, 6, 5, 4,
    8, 8, 8, 8, 8, 8, 8, 8,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9,
];

#[repr(align(16))]
#[derive(Clone, Copy)]
struct SparseEntry {
    indexes: [u16; 8],
    count: usize,
}

#[derive(Clone)]
pub struct Network {
    index: usize,
    stack: Box<[Accumulator]>,
    cache: AccumulatorCache,
    nnz_table: Box<[SparseEntry]>,
    parameters: &'static Parameters,
}

impl Network {
    pub fn push(&mut self, mv: Move, board: &Board) {
        debug_assert!(mv.is_some());

        self.index += 1;
        self.stack[self.index].accurate = [false; 2];
        self.stack[self.index].delta.mv = mv;
        self.stack[self.index].delta.piece = board.piece_on(mv.from());
        self.stack[self.index].delta.captured = board.piece_on(mv.to());
    }

    pub fn pop(&mut self) {
        self.index -= 1;
    }

    pub fn full_refresh(&mut self, board: &Board) {
        self.refresh(board, Color::White);
        self.refresh(board, Color::Black);
    }

    pub fn evaluate(&mut self, board: &Board) -> i32 {
        debug_assert!(self.stack[0].accurate == [true; 2]);

        for pov in [Color::White, Color::Black] {
            if self.stack[self.index].accurate[pov] {
                continue;
            }

            if self.can_update(pov) {
                self.update_accumulator(board, pov);
            } else {
                self.refresh(board, pov);
            }
        }

        self.output_transformer(board)
    }

    fn refresh(&mut self, board: &Board, pov: Color) {
        self.stack[self.index].refresh(self.parameters, board, pov, &mut self.cache);
    }

    fn update_accumulator(&mut self, board: &Board, pov: Color) {
        let king = board.king_square(pov);
        let index = (0..self.index).rfind(|&i| self.stack[i].accurate[pov]).unwrap();

        for i in index..self.index {
            if let (prev, [current, ..]) = self.stack.split_at_mut(i + 1) {
                current.update(self.parameters, &prev[i], board, king, pov);
            }
        }
    }

    fn can_update(&self, pov: Color) -> bool {
        for i in (0..=self.index).rev() {
            let delta = &self.stack[i].delta;

            let (from, to) = match delta.piece.piece_color() {
                Color::White => (delta.mv.from(), delta.mv.to()),
                Color::Black => (delta.mv.from() ^ 56, delta.mv.to() ^ 56),
            };

            if delta.piece.piece_type() == PieceType::King
                && delta.piece.piece_color() == pov
                && ((from.file() >= 4) != (to.file() >= 4) || BUCKETS[from] != BUCKETS[to])
            {
                return false;
            }

            if self.stack[i].accurate[pov] {
                return true;
            }
        }

        false
    }

    fn output_transformer(&self, board: &Board) -> i32 {
        unsafe {
            let ft_out = forward::activate_ft(&self.stack[self.index], board.side_to_move());
            let (nnz_indexes, nnz_count) = forward::find_nnz(&ft_out, &self.nnz_table);

            let l1_out = forward::propagate_l1(self.parameters, ft_out, &nnz_indexes[..nnz_count]);
            let l2_out = forward::propagate_l2(self.parameters, l1_out);
            let l3_out = forward::propagate_l3(self.parameters, l2_out);

            (l3_out * NETWORK_SCALE as f32) as i32
        }
    }
}

impl Default for Network {
    fn default() -> Self {
        let parameters = load_parameters();
        let mut nnz_table = vec![SparseEntry { indexes: [0; 8], count: 0 }; 256];

        for (byte, entry) in nnz_table.iter_mut().enumerate() {
            let mut count = 0;

            for bit in 0..8 {
                if (byte & (1 << bit)) != 0 {
                    entry.indexes[count] = bit as u16;
                    count += 1;
                }
            }

            entry.count = count;
        }

        Self {
            index: 0,
            stack: vec![Accumulator::new(parameters); MAX_PLY].into_boxed_slice(),
            cache: AccumulatorCache::new(parameters),
            nnz_table: nnz_table.into_boxed_slice(),
            parameters,
        }
    }
}

#[repr(C)]
pub struct Parameters {
    ft_weights: Aligned<[[i16; L1_SIZE]; INPUT_BUCKETS * FT_SIZE]>,
    ft_biases: Aligned<[i16; L1_SIZE]>,
    l1_weights: Aligned<[i8; L2_SIZE * L1_SIZE]>,
    l1_biases: Aligned<[f32; L2_SIZE]>,
    l2_weights: Aligned<[[f32; L3_SIZE]; L2_SIZE]>,
    l2_biases: Aligned<[f32; L3_SIZE]>,
    l3_weights: Aligned<[f32; L3_SIZE]>,
    l3_biases: f32,
}

fn get_current_cpu_and_node() -> usize {
    unsafe {
        let mut cpu: c_uint = 0;
        let mut node: c_uint = 0;

        match syscall(SYS_getcpu, &mut cpu, &mut node, std::ptr::null_mut::<libc::c_void>()) {
            0 => node as usize,
            _ => 0,
        }
    }
}

pub fn load_parameters() -> &'static Parameters {
    const MAX_NODES: usize = 4;

    static EMBEDDED: &[u8] = include_bytes!(concat!(env!("MODEL")));
    static CACHED: OnceLock<Mutex<Vec<Option<Arc<Mmap>>>>> = OnceLock::new();

    let node = get_current_cpu_and_node() % MAX_NODES;
    let cached = CACHED.get_or_init(|| Mutex::new(vec![None; MAX_NODES]));

    let mut guard = cached.lock().unwrap();
    let mmap = guard[node].get_or_insert_with(|| {
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(EMBEDDED).unwrap();
        tmpfile.flush().unwrap();

        let file = tmpfile.as_file();
        unsafe { Arc::new(Mmap::map(file).unwrap()) }
    });

    unsafe { &*(mmap.as_ptr().cast()) }
}

#[repr(align(64))]
#[derive(Copy, Clone)]
pub struct Aligned<T> {
    data: T,
}

impl<T> Aligned<T> {
    pub const fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T> std::ops::Deref for Aligned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> std::ops::DerefMut for Aligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
