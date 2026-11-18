//! Layered entries, shared by DTC and DTM50: a value per position per clock
//! layer, and the DTC and DTM50 reads.

use std::{cell::RefCell, sync::Arc};

use crate::types::Color;

use super::{
    cache::{TlFifo, fetch_block},
    common::*,
    compress::lzma_decompress_into,
    memory::read_u32,
    tables::{LayeredPerColor, TableFile, dtm_value_from_storage, rank_value},
};

pub const DTM50_PACK_LAYERS: usize = DTM50_HMC_COUNT as usize + 1;
pub const DTC_BUDGET_LAYERS: usize = 30;
pub const DTC_PACK_LAYERS: usize = DTC_BUDGET_LAYERS + 1;

const fn layered_bitmap_bytes(layers: usize) -> usize {
    layers.div_ceil(32) * 4
}

const DTM50_MULTI_BITMAP_BYTES: usize = layered_bitmap_bytes(DTM50_PACK_LAYERS);
const DTC_MULTI_BITMAP_BYTES: usize = layered_bitmap_bytes(DTC_PACK_LAYERS);

pub type DtcFile = TableFile<LayeredPerColor>;
pub type Dtm50File = TableFile<LayeredPerColor>;

thread_local! {
    static DTC_BLOCKS: RefCell<Vec<TlFifo<LayeredBlock>>> = RefCell::new(TlFifo::sharded(1, 16));
    static DTM50_BLOCKS: RefCell<Vec<TlFifo<LayeredBlock>>> = RefCell::new(TlFifo::sharded(1, 16));
}

/// The plies a clock leaves before the 50-move rule draws.
pub const fn dtc_budget_plies(rule50: u32) -> u32 {
    if rule50 == IGNORE_50MR { DTZ_MAX_NON_CURSED as u32 } else { (DTZ_MAX_NON_CURSED as u32).saturating_sub(rule50) }
}

/// `bytes[at..at + n]` (n <= 8) as the low bytes of a word.
fn read_partial(bytes: &[u8], at: usize, n: usize) -> u64 {
    let mut w = [0u8; 8];
    w[..n].copy_from_slice(&bytes[at..at + n]);
    u64::from_le_bytes(w)
}

/// Set bits of `bm` in [lo, hi).
fn popcount_bit_range(bm: &[u8], lo: usize, hi: usize) -> usize {
    if lo >= hi {
        return 0;
    }
    let mut pc = 0;
    let mut byte = lo >> 3;
    let mut remaining = hi - lo;
    let head = lo & 7; // bits to skip in the first byte
    if head != 0 {
        let b = bm[byte] >> head;
        let avail = 8 - head;
        if remaining <= avail {
            return (b & ((1u16 << remaining) - 1) as u8).count_ones() as usize;
        }
        pc += b.count_ones() as usize;
        remaining -= avail;
        byte += 1;
    }
    while remaining >= 64 {
        pc += read_partial(bm, byte, 8).count_ones() as usize;
        byte += 8;
        remaining -= 64;
    }
    if remaining != 0 {
        let w = read_partial(bm, byte, remaining.div_ceil(8)) & ((1u64 << remaining) - 1);
        pc += w.count_ones() as usize;
    }
    pc
}

const LAYERED_PREFIX_STRIDE: usize = 256;

/// Counts of each state's records before a stride.
#[derive(Copy, Clone, Default)]
struct PrefixEntry {
    n_const: u32,
    n_single: u32,
    n_double: u32,
    n_multi: u32,
    n_single_short: u32, // popcount(single_hints, [0, n_single))
    n_double_short: u32, // popcount(double_hints, [0, n_double))
}

/// Position p's state is bits 2p and 2p+1 of the state vector: 00 CONST,
/// 01 SINGLE, 10 DOUBLE, 11 MULTI. NONCONST counts all but 00.
#[derive(Copy, Clone)]
enum StateClass {
    Single,
    Double,
    Multi,
    NonConst,
}

fn tally_state_word(cls: StateClass, w: u64) -> u32 {
    const EVEN_MASK: u64 = 0x5555_5555_5555_5555;
    let lo = w & EVEN_MASK;
    let hi = (w >> 1) & EVEN_MASK;
    match cls {
        StateClass::Single => (lo & !hi).count_ones(),
        StateClass::Double => (hi & !lo).count_ones(),
        StateClass::Multi => (lo & hi).count_ones(),
        StateClass::NonConst => (lo | hi).count_ones(),
    }
}

/// `f` on each 64-bit word of the states in [bit_lo, bit_hi), the last one
/// masked to the range.
fn for_state_words(sb: &[u8], bit_lo: usize, bit_hi: usize, mut f: impl FnMut(u64)) {
    let mut bit = bit_lo;
    while bit + 64 <= bit_hi {
        f(read_partial(sb, bit / 8, 8));
        bit += 64;
    }
    if bit < bit_hi {
        let rem = bit_hi - bit;
        f(read_partial(sb, bit / 8, rem.div_ceil(8)) & ((1u64 << rem) - 1));
    }
}

fn popcount_state_class(cls: StateClass, sb: &[u8], bit_lo: usize, bit_hi: usize) -> usize {
    let mut count = 0;
    for_state_words(sb, bit_lo, bit_hi, |w| count += tally_state_word(cls, w) as usize);
    count
}

/// A decoded layered block: the payload, with each record stream's offset,
/// the draw-end hint bits of the SINGLE and DOUBLE records, and per-stride
/// record counts.
pub struct LayeredBlock {
    payload: Vec<u8>,
    state_bits_off: usize,
    const_stream_off: usize,
    single_stream_off: usize,
    double_stream_off: usize,
    multi_dir_off: usize,
    multi_data_off: usize,
    prefix: Vec<PrefixEntry>,
    single_hints: Vec<u8>,
    double_hints: Vec<u8>,
}

fn layered_get_block(pc: &LayeredPerColor, data: &[u8], block_id: usize) -> (LayeredBlock, usize) {
    let [lo, hi] = pc.b.offsets.get2(data, block_id);
    let usz = pc.usizes.get(data, block_id) as usize;
    debug_assert!(usz.trailing_zeros() >= 2);

    let num_positions =
        if block_id == pc.b.block_cnt - 1 && pc.tail_positions != 0 { pc.tail_positions } else { pc.block_positions };

    let mut payload = vec![0u8; usz];
    let ok = lzma_decompress_into(
        &mut payload,
        &data[pc.b.compressed_data + lo as usize..pc.b.compressed_data + hi as usize],
    );
    debug_assert!(ok, "corrupt LZMA block");

    let num_single = read_u32(&payload, 4) as usize;
    let num_double = read_u32(&payload, 8) as usize;
    let num_multi = read_u32(&payload, 12) as usize;
    let single_stream_bytes = read_u32(&payload, 16) as usize;
    let double_stream_bytes = read_u32(&payload, 20) as usize;
    debug_assert_eq!(read_u32(&payload, 0) as usize, num_positions);

    let eb = pc.entry_bytes;
    let num_const = num_positions - num_single - num_double - num_multi;
    let sb_bytes = (num_positions * 2).div_ceil(8);

    let mut p = 24;
    let state_bits_off = p;
    p += sb_bytes;
    let const_stream_off = p;
    p += num_const * eb;
    let single_stream_off = p;
    p += single_stream_bytes;
    let double_stream_off = p;
    p += double_stream_bytes;
    p += (4 - (p & 3)) & 3;
    let multi_dir_off = p;
    p += (num_multi + 1) * 4;
    let multi_data_off = p;

    let mut single_hints = vec![0u8; num_single.div_ceil(8)];
    let mut e = single_stream_off;
    for j in 0..num_single {
        let short = payload[e] & 0x80 != 0;
        if short {
            single_hints[j >> 3] |= 1 << (j & 7);
        }
        e += if short { 1 + eb } else { 1 + 2 * eb };
    }
    let mut double_hints = vec![0u8; num_double.div_ceil(8)];
    let mut e = double_stream_off;
    for j in 0..num_double {
        let short = payload[e + 1] & 0x80 != 0;
        if short {
            double_hints[j >> 3] |= 1 << (j & 7);
        }
        e += if short { 2 + 2 * eb } else { 2 + 3 * eb };
    }

    // Per stride, the counts of each state before it, and of the short hints.
    let n_strides = num_positions.div_ceil(LAYERED_PREFIX_STRIDE);
    let mut prefix = Vec::with_capacity(n_strides);
    let (mut nc, mut ns, mut nd, mut nm, mut ns_short, mut nd_short) = (0u32, 0u32, 0u32, 0u32, 0u32, 0u32);
    let sb = &payload[state_bits_off..];
    for s in 0..n_strides {
        prefix.push(PrefixEntry {
            n_const: nc,
            n_single: ns,
            n_double: nd,
            n_multi: nm,
            n_single_short: ns_short,
            n_double_short: nd_short,
        });
        let p_lo = s * LAYERED_PREFIX_STRIDE;
        let p_hi = (p_lo + LAYERED_PREFIX_STRIDE).min(num_positions);
        let (ns_before, nd_before, nm_before) = (ns, nd, nm);
        for_state_words(sb, p_lo * 2, p_hi * 2, |w| {
            ns += tally_state_word(StateClass::Single, w);
            nd += tally_state_word(StateClass::Double, w);
            nm += tally_state_word(StateClass::Multi, w);
        });
        let added_nonc = (ns - ns_before) + (nd - nd_before) + (nm - nm_before);
        nc += (p_hi - p_lo) as u32 - added_nonc;
        ns_short += popcount_bit_range(&single_hints, ns_before as usize, ns as usize) as u32;
        nd_short += popcount_bit_range(&double_hints, nd_before as usize, nd as usize) as u32;
    }

    let bytes = 36 + usz + n_strides * 24 + single_hints.len() + double_hints.len();
    let block = LayeredBlock {
        payload,
        state_bits_off,
        const_stream_off,
        single_stream_off,
        double_stream_off,
        multi_dir_off,
        multi_data_off,
        prefix,
        single_hints,
        double_hints,
    };
    (block, bytes)
}

/// A position's packed record: `entry` is its offset in the payload (the rank
/// cell for CONST), `state` 0 CONST, 1 SINGLE, 2 DOUBLE, 3 MULTI.
struct EntryRef {
    block: Arc<LayeredBlock>,
    entry: usize,
    state: u8,
    draw_end: bool,
}

impl EntryRef {
    fn bytes(&self) -> &[u8] {
        &self.block.payload[self.entry..]
    }
}

type BlockTl = std::thread::LocalKey<RefCell<Vec<TlFifo<LayeredBlock>>>>;

fn layered_locate_entry(pc: &LayeredPerColor, data: &[u8], pos: usize, tl: &'static BlockTl) -> Option<EntryRef> {
    let ppb = pc.block_positions;
    let block_id = pos / ppb;
    let pos_in_block = pos % ppb;

    let [lo, hi] = pc.b.offsets.get2(data, block_id);
    if lo == hi {
        return None;
    }
    let block = fetch_block(tl, &pc.b.cache, block_id, true, || layered_get_block(pc, data, block_id));
    let b = &*block;
    let eb = pc.entry_bytes;

    // The position's state, and its index among its state's records: the
    // stride's count plus those of its state since the stride began.
    let sb = &b.payload[b.state_bits_off..];
    let bit_pos = pos_in_block * 2;
    let state = (sb[bit_pos / 8] >> (bit_pos % 8)) & 3;
    let stride = pos_in_block / LAYERED_PREFIX_STRIDE;
    let snap = b.prefix[stride];
    let bit_lo = stride * LAYERED_PREFIX_STRIDE * 2;
    let idx = match state {
        1 => snap.n_single as usize + popcount_state_class(StateClass::Single, sb, bit_lo, bit_pos),
        2 => snap.n_double as usize + popcount_state_class(StateClass::Double, sb, bit_lo, bit_pos),
        3 => snap.n_multi as usize + popcount_state_class(StateClass::Multi, sb, bit_lo, bit_pos),
        _ => {
            snap.n_const as usize + (bit_pos - bit_lo) / 2
                - popcount_state_class(StateClass::NonConst, sb, bit_lo, bit_pos)
        }
    };

    let (entry, draw_end) = match state {
        0 => (b.const_stream_off + idx * eb, false),
        1 => {
            // Short = draw-end variant: [h, r0] against [h, r0, r1].
            let hints = &b.single_hints;
            let n_short = snap.n_single_short as usize + popcount_bit_range(hints, snap.n_single as usize, idx);
            let byte_off = n_short * (1 + eb) + (idx - n_short) * (1 + 2 * eb);
            (b.single_stream_off + byte_off, (hints[idx >> 3] >> (idx & 7)) & 1 != 0)
        }
        2 => {
            let hints = &b.double_hints;
            let n_short = snap.n_double_short as usize + popcount_bit_range(hints, snap.n_double as usize, idx);
            let byte_off = n_short * (2 + 2 * eb) + (idx - n_short) * (2 + 3 * eb);
            (b.double_stream_off + byte_off, (hints[idx >> 3] >> (idx & 7)) & 1 != 0)
        }
        _ => {
            let entry = b.multi_data_off + read_u32(&b.payload, b.multi_dir_off + idx * 4) as usize;
            (entry, b.payload[entry] & 0x80 != 0)
        }
    };
    Some(EntryRef { block, entry, state, draw_end })
}

/// A MULTI record's changepoint bitmap, the `bm_bytes` after its k byte, in
/// words: word w bit b is layer 64 * w + b.
fn layered_multi_bitmap(entry: &[u8], bm_bytes: usize) -> [u64; 2] {
    let mut bm = [0u64; 2];
    for (w, word) in bm.iter_mut().enumerate().take(bm_bytes.div_ceil(8)) {
        let n = (bm_bytes - w * 8).min(8);
        *word = read_partial(entry, 1 + w * 8, n);
    }
    bm
}

/// Index of a MULTI record's highest set changepoint bit.
fn layered_multi_last_changepoint(entry: &[u8], bm_bytes: usize) -> u16 {
    let bm = layered_multi_bitmap(entry, bm_bytes);
    (0..bm.len()).rev().find(|&w| bm[w] != 0).map_or(0, |w| (w * 64 + 63 - bm[w].leading_zeros() as usize) as u16)
}

/// Rank slot selected at `layer`: one less than the number of changepoints at
/// or below it.
fn layered_multi_rank_slot(entry: &[u8], bm_bytes: usize, layer: u16) -> usize {
    let bm = layered_multi_bitmap(entry, bm_bytes);
    let layer = layer as usize;
    let mut pc = 0;
    for (w, &word) in bm.iter().enumerate().take(bm_bytes.div_ceil(8)) {
        if layer < w * 64 {
            break;
        }
        let bits = layer - w * 64 + 1;
        let mask = if bits >= 64 { !0 } else { (1u64 << bits) - 1 };
        pc += (word & mask).count_ones() as usize;
    }
    pc.wrapping_sub(1)
}

/// One DTC read. `order` is the pawn pushes owed before the conversion,
/// `value` the plies (DRAWN_AT_LAYER: none at the clock), `dtz` the cell's DTZ.
#[derive(Copy, Clone)]
pub struct DtcCell {
    pub order: u16,
    pub value: u16,
    pub dtz: u16,
    pub dtz_rounded: bool,
}

impl Default for DtcCell {
    fn default() -> Self {
        Self {
            order: 0,
            value: DRAWN_AT_LAYER,
            dtz: DRAWN_AT_LAYER,
            dtz_rounded: false,
        }
    }
}

impl DtcCell {
    pub const fn priced(self) -> bool {
        self.value != DRAWN_AT_LAYER
    }
}

/// A position's DTC per budget layer (`value[budget]`), DRAWN_AT_LAYER where
/// it draws.
pub struct DtcCurve {
    pub value: [u16; DTC_PACK_LAYERS],
}

/// A DTC record as segments of pack indices, each with its value; with
/// `draw_end`, the last segment is DRAW.
#[derive(Default)]
struct BudgetSegments {
    start: [u16; DTC_PACK_LAYERS],
    value: [u16; DTC_PACK_LAYERS],
    count: usize,
    draw_end: bool,
}

fn decode_segments(pc: &LayeredPerColor, r: &EntryRef) -> BudgetSegments {
    let eb = pc.entry_bytes;
    let e = r.bytes();
    let rank = |at: usize| rank_value(&pc.rank_to_value, eb, e, at);
    let mut out = BudgetSegments { draw_end: r.draw_end, ..Default::default() };
    match r.state {
        0 => {
            out.value[0] = rank(0);
            out.count = 1;
        }
        1 => {
            out.value[0] = rank(1);
            out.start[1] = (e[0] & 0x7F) as u16;
            out.count = 2;
            if !r.draw_end {
                out.value[1] = rank(1 + eb);
            }
        }
        2 => {
            out.value[0] = rank(2);
            out.start[1] = e[0] as u16;
            out.value[1] = rank(2 + eb);
            out.start[2] = (e[1] & 0x7F) as u16;
            out.count = 3;
            if !r.draw_end {
                out.value[2] = rank(2 + 2 * eb);
            }
        }
        _ => {
            let k = (e[0] & 0x7F) as usize;
            let words = layered_multi_bitmap(e, DTC_MULTI_BITMAP_BYTES);
            let ranks = 1 + DTC_MULTI_BITMAP_BYTES;
            let mut n = 0;
            for (w, &word) in words.iter().enumerate() {
                let mut bits = word;
                while bits != 0 {
                    let b = bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    out.start[n] = (w * 64 + b) as u16;
                    // The hint omits the last rank, DRAW standing in for it.
                    if !(r.draw_end && n == k - 1) {
                        out.value[n] = rank(ranks + n * eb);
                    }
                    n += 1;
                }
            }
            debug_assert_eq!(n, k);
            out.count = k;
        }
    }
    out
}

/// The record's segments; `None` where nothing is stored, which reads as
/// drawn at every budget.
fn decode_dtc_position(f: &DtcFile, c: Color, pos: View) -> Option<BudgetSegments> {
    if f.is_singular[c] {
        return None;
    }
    let pc = &f.per_color[c];
    let pos = f.storage_index(c, pos)?;
    let r = layered_locate_entry(pc, f.data(), pos, &DTC_BLOCKS)?;
    Some(decode_segments(pc, &r))
}

const fn budget_of_pack_index(j: usize) -> usize {
    DTC_BUDGET_LAYERS - j
}

/// DTC at clock `rule50` (IGNORE_50MR: unclocked), and the cell's DTZ.
pub fn read_dtc(f: &DtcFile, c: Color, pos: View, wdl: WdlEntry, rule50: u32) -> DtcCell {
    let Some(seg) = decode_dtc_position(f, c, pos) else { return DtcCell::default() };

    let mut out = DtcCell { dtz: seg.value[0], ..Default::default() };
    if matches!(wdl, WdlEntry::CursedWin | WdlEntry::BlessedLoss) {
        out.dtz_rounded = out.dtz & 1 != 0;
        return out;
    }

    let budget_plies = dtc_budget_plies(rule50);
    let decisive = if seg.draw_end { seg.count - 1 } else { seg.count };
    for s in (0..decisive).rev() {
        if seg.value[s] as u32 > budget_plies {
            continue;
        }
        let last_index = if s + 1 < seg.count { seg.start[s + 1] as usize - 1 } else { DTC_PACK_LAYERS - 1 };
        out.order = budget_of_pack_index(last_index) as u16;
        out.value = seg.value[s];
        break;
    }
    out
}

/// The DTC at every budget.
pub fn read_dtc_curve(f: &DtcFile, c: Color, pos: View, wdl: WdlEntry) -> DtcCurve {
    let mut curve = DtcCurve { value: [DRAWN_AT_LAYER; DTC_PACK_LAYERS] };
    if matches!(wdl, WdlEntry::CursedWin | WdlEntry::BlessedLoss) {
        return curve;
    }
    let Some(seg) = decode_dtc_position(f, c, pos) else { return curve };

    let decisive = if seg.draw_end { seg.count - 1 } else { seg.count };
    for s in 0..decisive {
        let end = if s + 1 < seg.count { seg.start[s + 1] as usize } else { DTC_PACK_LAYERS };
        for j in seg.start[s] as usize..end {
            curve.value[budget_of_pack_index(j)] = seg.value[s];
        }
    }
    curve
}

/// One DTM50 cell. `draw_flip` is the layer where the position turns DRAW, 0
/// if it never does; for a plain WIN/LOSE cell it pins DTZ. `value` is
/// DRAWN_AT_LAYER when the probed layer is DRAW.
#[derive(Copy, Clone)]
pub struct Dtm50Cell {
    pub value: u16,
    pub draw_flip: u16,
}

impl Default for Dtm50Cell {
    fn default() -> Self {
        Self { value: DRAWN_AT_LAYER, draw_flip: 0 }
    }
}

/// The value at `hmc` (IGNORE_50MR: the unclocked layer), and the layer where
/// the cell turns DRAW; the terminal rank decides both. Only the draw-end hint
/// says DRAW; a stored 0 is a mate.
pub fn read_dtm50(f: &Dtm50File, c: Color, pos: View, wdl: WdlEntry, hmc: u32) -> Dtm50Cell {
    let flat = hmc == IGNORE_50MR;
    if !flat && matches!(wdl, WdlEntry::CursedWin | WdlEntry::BlessedLoss) {
        return Dtm50Cell::default();
    }
    if f.is_singular[c] {
        return Dtm50Cell::default();
    }
    let pc = &f.per_color[c];
    let Some(pos) = f.storage_index(c, pos) else { return Dtm50Cell::default() };
    let Some(r) = layered_locate_entry(pc, f.data(), pos, &DTM50_BLOCKS) else { return Dtm50Cell::default() };

    let eb = pc.entry_bytes;
    let e = r.bytes();
    let rank = |at: usize| rank_value(&pc.rank_to_value, eb, e, at);

    let layer = if flat { 0 } else { hmc as u16 + 1 };
    let mut draw_flip = 0;
    let stored = match r.state {
        // CONST: one rank for all hmc.
        0 => rank(0),
        // SINGLE: one transition at h.
        1 => {
            let h = (e[0] & 0x7F) as u16;
            if r.draw_end {
                draw_flip = h;
            }
            if layer < h {
                rank(1)
            } else if r.draw_end {
                return Dtm50Cell { value: DRAWN_AT_LAYER, draw_flip };
            } else {
                rank(1 + eb)
            }
        }
        // DOUBLE: transitions at h1 < h2.
        2 => {
            let h1 = e[0] as u16;
            let h2 = (e[1] & 0x7F) as u16;
            if r.draw_end {
                draw_flip = h2;
            }
            if layer < h1 {
                rank(2)
            } else if layer < h2 {
                rank(2 + eb)
            } else if r.draw_end {
                return Dtm50Cell { value: DRAWN_AT_LAYER, draw_flip };
            } else {
                rank(2 + 2 * eb)
            }
        }
        // MULTI: changepoint bitmap; the slot is popcount(bits <= layer) - 1.
        _ => {
            let k = (e[0] & 0x7F) as usize;
            let rsel = layered_multi_rank_slot(e, DTM50_MULTI_BITMAP_BYTES, layer);
            if r.draw_end {
                draw_flip = layered_multi_last_changepoint(e, DTM50_MULTI_BITMAP_BYTES);
            }
            if rsel == k.wrapping_sub(1) && r.draw_end {
                return Dtm50Cell { value: DRAWN_AT_LAYER, draw_flip };
            }
            rank(1 + DTM50_MULTI_BITMAP_BYTES + rsel * eb)
        }
    };

    // Clocked, the cursed classes have returned: both layers value alike.
    Dtm50Cell { value: dtm_value_from_storage(stored, wdl), draw_flip }
}
