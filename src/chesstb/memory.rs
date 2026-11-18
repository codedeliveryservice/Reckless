//! A table file's header reader and its packed integer vectors: the block
//! offsets and sizes a table file stores.
//!
//! Bit reads are unaligned and read up to two 8-byte words, so every packed
//! region must be followed by >= 8 readable bytes; on disk, whatever follows
//! each section (more sections, the compressed data, the end checksum) is.

/// Serial little-endian reads over a mapped file; `None` past its end.
pub struct Reader<'a> {
    data: &'a [u8],
    caret: usize,
}

impl<'a> Reader<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, caret: 0 }
    }

    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let bytes = self.data.get(self.caret..self.caret + N)?.try_into().ok()?;
        self.caret += N;
        Some(bytes)
    }

    pub fn u8(&mut self) -> Option<u8> {
        self.take::<1>().map(|b| b[0])
    }

    pub fn u16(&mut self) -> Option<u16> {
        self.take().map(u16::from_le_bytes)
    }

    pub fn u32(&mut self) -> Option<u32> {
        self.take().map(u32::from_le_bytes)
    }

    pub fn u64(&mut self) -> Option<u64> {
        self.take().map(u64::from_le_bytes)
    }

    pub fn advance(&mut self, size: usize) -> Option<()> {
        (self.caret + size <= self.data.len()).then(|| self.caret += size)
    }

    pub fn align(&mut self, alignment: usize) -> Option<()> {
        let misalignment = self.caret % alignment;
        if misalignment == 0 { Some(()) } else { self.advance(alignment - misalignment) }
    }

    /// The offset of the next read.
    pub const fn caret(&self) -> usize {
        self.caret
    }
}

pub fn read_u64(data: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(data[at..at + 8].try_into().unwrap())
}

pub fn read_u32(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

pub fn read_u16(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(data[at..at + 2].try_into().unwrap())
}

const fn low_mask(width: u32) -> u64 {
    if width >= 64 { !0 } else { (1u64 << width) - 1 }
}

/// `width` (<= 64) bits from bit `bitpos` of `data[base..]`. Requires >= 8
/// readable bytes past the last touched byte.
fn read_bits(data: &[u8], base: usize, bitpos: usize, width: u32) -> u64 {
    if width == 0 {
        return 0;
    }
    let byte = base + (bitpos >> 3);
    let bit = (bitpos & 7) as u32;
    let mut v = read_u64(data, byte) >> bit;
    if bit + width > 64 {
        v |= read_u64(data, byte + 8) << (64 - bit);
    }
    v & low_mask(width)
}

/// Monotone offsets as a strided sample plus fixed-width deltas. On disk:
/// [sample bits][delta bits]; the width bytes live in the section header.
#[derive(Copy, Clone, Default)]
pub struct MonoUintVec {
    base: usize,
    delta: usize,
    log2_bu: u8,
    sample_width: u8,
    offset_width: u8,
}

impl MonoUintVec {
    /// `num_values` is the count of stored offsets (block count + 1).
    pub const fn new(blob: usize, num_values: usize, log2_bu: u8, sample_width: u8, offset_width: u8) -> Self {
        let num_samples = num_values.div_ceil(1 << log2_bu);
        let delta = blob + (num_samples * sample_width as usize).div_ceil(8);
        Self { base: blob, delta, log2_bu, sample_width, offset_width }
    }

    /// `{O[i], O[i+1]}`, for any block id `i`.
    pub fn get2(&self, data: &[u8], i: usize) -> [u64; 2] {
        let sb0 = i >> self.log2_bu;
        let sb1 = (i + 1) >> self.log2_bu;
        let ow = self.offset_width as u32;
        let sw = self.sample_width as u32;
        let d0 = read_bits(data, self.delta, i * ow as usize, ow);
        let d1 = read_bits(data, self.delta, (i + 1) * ow as usize, ow);
        let b0 = read_bits(data, self.base, sb0 * sw as usize, sw);
        let b1 = if sb1 == sb0 { b0 } else { read_bits(data, self.base, sb1 * sw as usize, sw) };
        [b0 + d0, b1 + d1]
    }

    pub const fn on_disk_bytes(num_values: usize, log2_bu: u8, sample_width: u8, offset_width: u8) -> usize {
        let num_samples = num_values.div_ceil(1 << log2_bu);
        (num_samples * sample_width as usize).div_ceil(8) + (num_values * offset_width as usize).div_ceil(8)
    }
}

/// Fixed-width values from zero.
#[derive(Copy, Clone, Default)]
pub struct Min0UintVec {
    data: usize,
    width: u8,
}

impl Min0UintVec {
    pub const fn new(data: usize, width: u8) -> Self {
        Self { data, width }
    }

    pub fn get(&self, data: &[u8], i: usize) -> u64 {
        read_bits(data, self.data, i * self.width as usize, self.width as u32)
    }

    pub const fn on_disk_bytes(size: usize, width: u8) -> usize {
        (size * width as usize).div_ceil(8)
    }
}
