//! Table files: the header every metric shares, each metric's per-colour data,
//! and the WDL, DTZ and DTM reads (DTC and DTM50 are in `layered`).

use std::{cell::RefCell, path::Path, sync::Arc};

use crate::types::Color;

use super::{
    cache::{BlockCache, BlockPool, TlFifo, fetch_block},
    common::*,
    compress::{Lz4Checkpoint, lz4_build_point_index, lz4_point_read, lzma_decompress_into},
    filesystem::MappedFile,
    index::{IndexStorageLayout, PieceConfig, PositionIndexConfig},
    memory::{Min0UintVec, MonoUintVec, Reader, read_u16},
};

pub const WDL_EXT: &str = ".lzw";
pub const DTZ_EXT: &str = ".lzdtz";
pub const DTC_EXT: &str = ".lzdtc";
pub const DTM_EXT: &str = ".lzdtm";
pub const DTM50_EXT: &str = ".lzdtm50";

const SINGULAR_FLAG: u8 = 0x80;
const DROPPED_FLAG: u8 = 0x40;
const LOSS_ONLY_FLAG: u8 = 0x20;
const RELAXED_FLAG: u8 = 0x10;

thread_local! {
    static WDL_BLOCKS: RefCell<Vec<TlFifo<Vec<Lz4Checkpoint>>>> = RefCell::new(TlFifo::sharded(64, 128));
    static DTZ_BLOCKS: RefCell<Vec<TlFifo<Vec<u8>>>> = RefCell::new(TlFifo::sharded(1, 16));
    static DTM_BLOCKS: RefCell<Vec<TlFifo<Vec<u8>>>> = RefCell::new(TlFifo::sharded(1, 16));
}

/// What every stored colour has: its block cache, index layout, block offsets
/// and compressed data.
pub struct Blocks {
    pub cache: BlockCache,
    pub layout: IndexStorageLayout,
    pub block_cnt: usize,
    pub offsets: MonoUintVec,
    pub compressed_data: usize,
    pub data_size: usize,
}

/// A colour's data in one table file: its header and its blocks. A header
/// that doesn't parse reads as `None`.
pub trait PerColor: Sized {
    fn new(blocks: Blocks) -> Self;
    fn blocks(&self) -> &Blocks;
    fn blocks_mut(&mut self) -> &mut Blocks;
    fn on_singular(r: &mut Reader, pc: &mut Self) -> Option<()>;
    fn parse_header(r: &mut Reader, pc: &mut Self, cfg: &PositionIndexConfig) -> Option<()>;

    /// The sections before the offsets (WDL's dictionaries).
    fn read_prefix(_: &mut Reader, _: &mut [Self; 2], _: [bool; 2]) -> Option<()> {
        Some(())
    }

    /// The sections after a colour's offsets, whose header gave `usz_width`.
    fn read_after_offsets(_: &mut Reader, _: &mut Self, _: u8) -> Option<()> {
        Some(())
    }

    /// The sections after the colours' headers, `skip` the colours not stored.
    fn finalize(r: &mut Reader, per_color: &mut [Self; 2], skip: [bool; 2]) -> Option<()> {
        Self::read_prefix(r, per_color, skip)?;
        for c in [Color::White, Color::Black] {
            if skip[c] {
                continue;
            }
            let (offsets, usz_width) = read_offsets(r, per_color[c].blocks().block_cnt)?;
            per_color[c].blocks_mut().offsets = offsets;
            Self::read_after_offsets(r, &mut per_color[c], usz_width)?;
        }
        // The compressed data of each stored colour, in order, 64-byte aligned.
        for c in [Color::White, Color::Black] {
            if skip[c] {
                continue;
            }
            r.align(64)?;
            let b = per_color[c].blocks_mut();
            b.compressed_data = r.caret();
            r.advance(b.data_size)?;
        }
        Some(())
    }
}

/// One open table file. A colour is singular (one value throughout), dropped
/// (not stored: the other colour's mirror, or derived by search), or stored.
pub struct TableFile<P> {
    pub is_singular: [bool; 2],
    pub is_dropped: [bool; 2],
    pub is_loss_only: [bool; 2],
    pub is_relaxed: [bool; 2],
    pub per_color: [P; 2],
    index_cfg: PositionIndexConfig,
    map: MappedFile,
}

impl<P: PerColor> TableFile<P> {
    /// The file at `path` if it is a table of `magic` for material `ps`.
    pub fn load(ps: &PieceConfig, path: &Path, magic: u32, pool: &Arc<BlockPool>) -> Option<Self> {
        let map = MappedFile::open(path)?;
        let input = map.data();
        if input.len() & 63 != 8 {
            return None;
        }
        let mut reader = Reader::new(input);
        if reader.u32()? != magic {
            return None;
        }
        let key_and_table_num = reader.u32()?;
        if key_and_table_num >> 2 != ps.min_material_key().value() & 0x3FFF_FFFF {
            return None;
        }
        let index_cfg = PositionIndexConfig::new(ps);

        // Frames stored: white, and black unless the material is symmetric (1).
        let table_num = (key_and_table_num & 3) as usize;
        if table_num == 0 || table_num > 2 {
            return None;
        }

        let blocks = || Blocks {
            cache: BlockCache::new(pool.clone()),
            layout: IndexStorageLayout::default(),
            block_cnt: 0,
            offsets: MonoUintVec::default(),
            compressed_data: 0,
            data_size: 0,
        };
        let mut per_color = [P::new(blocks()), P::new(blocks())];
        let mut is_singular = [false; 2];
        let mut is_dropped = [false; 2];
        let mut is_loss_only = [false; 2];
        let mut is_relaxed = [false; 2];
        for c in [Color::White, Color::Black].into_iter().take(table_num) {
            let flag = reader.u8()?;
            is_loss_only[c] = flag & LOSS_ONLY_FLAG != 0;
            is_relaxed[c] = flag & RELAXED_FLAG != 0;
            if flag & SINGULAR_FLAG != 0 {
                is_singular[c] = true;
                P::on_singular(&mut reader, &mut per_color[c])?;
            } else if flag & DROPPED_FLAG != 0 {
                is_dropped[c] = true;
            } else {
                P::parse_header(&mut reader, &mut per_color[c], &index_cfg)?;
            }
        }
        if table_num == 1 {
            // Symmetric material: black is white mirrored, flag byte and all.
            is_dropped[Color::Black] = true;
            is_loss_only[Color::Black] = is_loss_only[Color::White];
            is_relaxed[Color::Black] = is_relaxed[Color::White];
        }

        let skip = [is_singular[0] || is_dropped[0], is_singular[1] || is_dropped[1]];
        P::finalize(&mut reader, &mut per_color, skip)?;
        Some(Self {
            is_singular,
            is_dropped,
            is_loss_only,
            is_relaxed,
            per_color,
            index_cfg,
            map,
        })
    }

    pub fn data(&self) -> &[u8] {
        self.map.data()
    }

    /// The position's index in colour `c`'s frame: 0 for a singular colour,
    /// `None` where the table has no such position.
    pub fn storage_index(&self, c: Color, pos: View) -> Option<usize> {
        debug_assert!(!self.is_dropped[c]);
        if self.is_singular[c] {
            return Some(0);
        }
        self.index_cfg.board_index(&self.per_color[c].blocks().layout, pos)
    }
}

/// The colour's index layout: a permutation of the piece classes.
fn read_layout(r: &mut Reader, cfg: &PositionIndexConfig) -> Option<IndexStorageLayout> {
    let perm = r.u32()?;
    cfg.permutation_is_valid(perm).then(|| cfg.storage_layout(perm))
}

/// A colour's offset section header and offset vector, and its `usz_width`.
fn read_offsets(r: &mut Reader, block_cnt: usize) -> Option<(MonoUintVec, u8)> {
    let log2_bu = r.u8()?;
    let sample_width = r.u8()?;
    let offset_width = r.u8()?;
    let usz_width = r.u8()?;
    let mono = MonoUintVec::new(r.caret(), block_cnt + 1, log2_bu, sample_width, offset_width);
    r.advance(MonoUintVec::on_disk_bytes(block_cnt + 1, log2_bu, sample_width, offset_width))?;
    Some((mono, usz_width))
}

/// A rank's width in bytes: 1 or 2.
fn read_entry_bytes(r: &mut Reader) -> Option<usize> {
    let entry_bytes = r.u8()? as usize;
    (entry_bytes == 1 || entry_bytes == 2).then_some(entry_bytes)
}

fn read_ranks(r: &mut Reader) -> Option<Vec<u16>> {
    let num_ranks = r.u16()? as usize;
    (0..num_ranks).map(|_| r.u16()).collect()
}

pub struct WdlPerColor {
    b: Blocks,
    block_size: usize,
    tail_size: usize,
    dict: (usize, usize),
    single_val: WdlStored,
}

impl PerColor for WdlPerColor {
    fn new(b: Blocks) -> Self {
        Self {
            b,
            block_size: 0,
            tail_size: 0,
            dict: (0, 0),
            single_val: WdlStored::Draw,
        }
    }

    fn blocks(&self) -> &Blocks {
        &self.b
    }

    fn blocks_mut(&mut self) -> &mut Blocks {
        &mut self.b
    }

    fn on_singular(r: &mut Reader, pc: &mut Self) -> Option<()> {
        pc.single_val = WdlStored::from_bits(r.u8()?);
        Some(())
    }

    fn parse_header(r: &mut Reader, pc: &mut Self, cfg: &PositionIndexConfig) -> Option<()> {
        pc.b.layout = read_layout(r, cfg)?;
        pc.tail_size = r.u16()? as usize;
        pc.block_size = r.u32()? as usize;
        pc.b.block_cnt = r.u64()? as usize;
        pc.b.data_size = r.u64()? as usize;
        (pc.block_size != 0).then_some(())
    }

    fn read_prefix(r: &mut Reader, per_color: &mut [Self; 2], skip: [bool; 2]) -> Option<()> {
        for c in [Color::White, Color::Black] {
            if skip[c] {
                continue;
            }
            let dict_size = r.u16()? as usize;
            per_color[c].dict = (r.caret(), dict_size);
            r.advance(dict_size)?;
        }
        Some(())
    }
}

pub type WdlFile = TableFile<WdlPerColor>;

impl WdlFile {
    /// The stored code of the position in colour `c`'s frame.
    pub fn read(&self, c: Color, pos: View) -> WdlStored {
        let pc = &self.per_color[c];
        if self.is_singular[c] {
            return pc.single_val;
        }
        let Some(pos) = self.storage_index(c, pos) else {
            return WdlStored::Illegal;
        };
        let data = self.data();

        let packed_byte = pos / WDL_ENTRY_PACK_RATIO;
        let block_id = packed_byte / pc.block_size;
        let in_block = packed_byte % pc.block_size;

        let [lo, hi] = pc.b.offsets.get2(data, block_id);
        if lo == hi {
            return WdlStored::Illegal;
        }
        let src = &data[pc.b.compressed_data + lo as usize..pc.b.compressed_data + hi as usize];
        let index = fetch_block(&WDL_BLOCKS, &pc.b.cache, block_id, false, || {
            let out_sz = if block_id == pc.b.block_cnt - 1 && pc.tail_size != 0 { pc.tail_size } else { pc.block_size };
            (lz4_build_point_index(src, out_sz), 0)
        });
        let dict = &data[pc.dict.0..pc.dict.0 + pc.dict.1];
        let entry = lz4_point_read(src, dict, &index, in_block);
        WdlStored::from_bits((entry >> ((pos % WDL_ENTRY_PACK_RATIO) * WDL_ENTRY_BITS)) & 0xF)
    }
}

/// DTZ and DTM: one LZMA block of fixed-width ranks per block of positions.
pub struct RankPerColor {
    b: Blocks,
    entry_bytes: usize,
    block_size: usize,
    tail_size: usize,
    rank_to_value: Vec<u16>,
    single_val: u16,
}

impl PerColor for RankPerColor {
    fn new(b: Blocks) -> Self {
        Self {
            b,
            entry_bytes: 0,
            block_size: 0,
            tail_size: 0,
            rank_to_value: Vec::new(),
            single_val: 0,
        }
    }

    fn blocks(&self) -> &Blocks {
        &self.b
    }

    fn blocks_mut(&mut self) -> &mut Blocks {
        &mut self.b
    }

    fn on_singular(r: &mut Reader, pc: &mut Self) -> Option<()> {
        pc.single_val = r.u8()? as u16;
        Some(())
    }

    fn parse_header(r: &mut Reader, pc: &mut Self, cfg: &PositionIndexConfig) -> Option<()> {
        pc.b.layout = read_layout(r, cfg)?;
        pc.entry_bytes = read_entry_bytes(r)?;
        pc.tail_size = r.u32()? as usize;
        pc.block_size = r.u32()? as usize;
        pc.b.block_cnt = r.u64()? as usize;
        pc.b.data_size = r.u64()? as usize;
        pc.rank_to_value = read_ranks(r)?;
        (pc.block_size >= pc.entry_bytes).then_some(())
    }
}

/// The value of the rank stored at `at`, `entry_bytes` wide.
pub fn rank_value(rank_to_value: &[u16], entry_bytes: usize, block: &[u8], at: usize) -> u16 {
    let rank = if entry_bytes == 1 { block[at] as usize } else { read_u16(block, at) as usize };
    rank_to_value[rank]
}

impl RankPerColor {
    /// The stored rank of position `pos`, `None` in a skipped block (uniform
    /// DRAW/ILLEGAL: the WDL class has already filtered those out; W/L would
    /// have forced a non-zero cell).
    fn read_rank(
        &self, data: &[u8], pos: usize, tl: &'static std::thread::LocalKey<RefCell<Vec<TlFifo<Vec<u8>>>>>,
    ) -> Option<u16> {
        let positions_per_block = self.block_size / self.entry_bytes;
        let block_id = pos / positions_per_block;
        let in_block_pos = pos % positions_per_block;

        let [lo, hi] = self.b.offsets.get2(data, block_id);
        if lo == hi {
            return None;
        }
        let block = fetch_block(tl, &self.b.cache, block_id, true, || {
            let decode_sz =
                if block_id == self.b.block_cnt - 1 && self.tail_size != 0 { self.tail_size } else { self.block_size };
            let mut buf = vec![0; decode_sz];
            let ok = lzma_decompress_into(
                &mut buf,
                &data[self.b.compressed_data + lo as usize..self.b.compressed_data + hi as usize],
            );
            debug_assert!(ok, "corrupt LZMA block");
            (buf, decode_sz)
        });
        Some(rank_value(&self.rank_to_value, self.entry_bytes, &block, in_block_pos * self.entry_bytes))
    }
}

#[derive(Copy, Clone, Default)]
pub struct DtzValue {
    pub dtz: u16,
    pub rounded: bool,
}

const fn dtz_value_from_storage(stored: u16, w: WdlEntry, entry_bytes: usize) -> DtzValue {
    if matches!(w, WdlEntry::Draw) {
        return DtzValue { dtz: 0, rounded: false };
    }
    if entry_bytes == 1 && matches!(w, WdlEntry::CursedWin | WdlEntry::BlessedLoss) {
        return DtzValue { dtz: (stored << 1).wrapping_sub(1), rounded: true };
    }
    DtzValue { dtz: stored, rounded: false }
}

pub type DtzFile = TableFile<RankPerColor>;
pub type DtmFile = TableFile<RankPerColor>;

/// DTZ in plies at a position of class `wdl`.
pub fn read_dtz(f: &DtzFile, c: Color, pos: View, wdl: WdlEntry) -> DtzValue {
    let pc = &f.per_color[c];
    if f.is_singular[c] {
        return dtz_value_from_storage(pc.single_val, wdl, 1);
    }
    let Some(pos) = f.storage_index(c, pos) else { return DtzValue::default() };
    match pc.read_rank(f.data(), pos, &DTZ_BLOCKS) {
        Some(stored) => dtz_value_from_storage(stored, wdl, pc.entry_bytes),
        None => DtzValue::default(),
    }
}

pub const fn dtm_value_from_storage(stored: u16, w: WdlEntry) -> u16 {
    match w {
        WdlEntry::Win | WdlEntry::CursedWin => (stored << 1) | 1,
        WdlEntry::Lose | WdlEntry::BlessedLoss => stored << 1,
        _ => 0,
    }
}

/// DTM in plies at a position of class `wdl`.
pub fn read_dtm(f: &DtmFile, c: Color, pos: View, wdl: WdlEntry) -> u16 {
    let pc = &f.per_color[c];
    if f.is_singular[c] {
        return dtm_value_from_storage(pc.single_val, wdl);
    }
    let Some(pos) = f.storage_index(c, pos) else { return 0 };
    pc.read_rank(f.data(), pos, &DTM_BLOCKS).map_or(0, |stored| dtm_value_from_storage(stored, wdl))
}

/// DTC and DTM50: per block of positions, an LZMA block of layered records.
pub struct LayeredPerColor {
    pub b: Blocks,
    pub entry_bytes: usize,
    pub block_positions: usize,
    pub tail_positions: usize,
    pub usizes: Min0UintVec,
    pub rank_to_value: Vec<u16>,
}

impl PerColor for LayeredPerColor {
    fn new(b: Blocks) -> Self {
        Self {
            b,
            entry_bytes: 0,
            block_positions: 0,
            tail_positions: 0,
            usizes: Min0UintVec::default(),
            rank_to_value: Vec::new(),
        }
    }

    fn blocks(&self) -> &Blocks {
        &self.b
    }

    fn blocks_mut(&mut self) -> &mut Blocks {
        &mut self.b
    }

    /// The singular value must be DRAW.
    fn on_singular(r: &mut Reader, _: &mut Self) -> Option<()> {
        (r.u8()? == 0).then_some(())
    }

    fn parse_header(r: &mut Reader, pc: &mut Self, cfg: &PositionIndexConfig) -> Option<()> {
        pc.b.layout = read_layout(r, cfg)?;
        pc.entry_bytes = read_entry_bytes(r)?;
        pc.block_positions = r.u32()? as usize;
        pc.b.block_cnt = r.u64()? as usize;
        pc.tail_positions = r.u32()? as usize;
        pc.b.data_size = r.u64()? as usize;
        pc.rank_to_value = read_ranks(r)?;
        (pc.block_positions != 0).then_some(())
    }

    /// The colour's unpacked block sizes.
    fn read_after_offsets(r: &mut Reader, pc: &mut Self, usz_width: u8) -> Option<()> {
        pc.usizes = Min0UintVec::new(r.caret(), usz_width);
        r.advance(Min0UintVec::on_disk_bytes(pc.b.block_cnt, usz_width))
    }
}
