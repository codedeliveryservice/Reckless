use memmap::*;

use std::{self, cell::UnsafeCell, collections::HashMap, fs, path::Path, slice, sync::{atomic::{AtomicBool, Ordering}, Mutex}};

use crate::{board::Board, types::{Bitboard, Color, MoveList, PieceType, Square}};

const TB_PIECES: usize = 7;

static mut MAX_CARDINALITY: u32 = 0;

pub fn max_cardinality() -> u32 {
    unsafe { MAX_CARDINALITY }
}

struct EncInfo {
    precomp: Option<Box<PairsData>>,
    factor: [usize; TB_PIECES],
    pieces: [u8; TB_PIECES],
    norm: [u8; TB_PIECES],
}

impl EncInfo {
    pub fn new() -> EncInfo {
        EncInfo {
            precomp: None,
            factor: [0; TB_PIECES],
            pieces: [0; TB_PIECES],
            norm: [0; TB_PIECES],
        }
    }
}

const WDL_TO_MAP: [u32; 5] = [1, 3, 0, 2, 0];
const PA_FLAGS: [u8; 5] = [ 8, 0, 0, 0, 4 ];

const WDL_MAGIC: u32 = 0x5d23e871;
const WDL_SUFFIX: &str = ".rtbw";

struct Wdl;

struct PieceEnc;
struct FileEnc;
struct RankEnc;

trait Encoding {
    const ENC: i32;
    type Entry: EntryInfo;
}

impl Encoding for PieceEnc {
    const ENC: i32 = 0;
    type Entry = PieceEntry;
}

impl Encoding for FileEnc {
    const ENC: i32 = 1;
    type Entry = PawnEntry;
}

impl Encoding for RankEnc {
    const ENC: i32 = 2;
    type Entry = PawnEntry;
}

trait TbType: Sized {
    type PieceTable: TbTable<Entry = PieceEntry, Type = Self>;
    type PawnTable: TbTable<Entry = PawnEntry, Type = Self>;
    type Select;
    const TYPE: i32;
    fn magic() -> u32;
    fn suffix() -> &'static str;
}

impl TbType for Wdl {
    type PieceTable = WdlPiece;
    type PawnTable = WdlPawn;
    type Select = ();
    const TYPE: i32 = 0;
    fn magic() -> u32 { WDL_MAGIC }
    fn suffix() -> &'static str { WDL_SUFFIX }
}

trait TbTable: Sized {
    type Type: TbType;
    type Entry: TbEntry<Self> + EntryInfo;
    type Enc: Encoding<Entry = Self::Entry>;
    fn mapping(&mut self) -> &mut Option<Box<Mmap>>;
    fn ready(&self) -> &AtomicBool;
    fn num_tables() -> usize;
    fn ei(&self, t: usize, idx: usize) -> &EncInfo;
    fn ei_mut(&mut self, t: usize, idx: usize) -> &mut EncInfo;
}

struct WdlPiece {
    mapping: Option<Box<Mmap>>,
    ei: [EncInfo; 2],
    ready: AtomicBool,
}

impl TbTable for WdlPiece {
    type Type = Wdl;
    type Entry = PieceEntry;
    type Enc = PieceEnc;
    fn mapping(&mut self) -> &mut Option<Box<Mmap>> { &mut self.mapping }
    fn ready(&self) -> &AtomicBool { &self.ready }
    fn num_tables() -> usize { 1 }
    fn ei(&self, _t: usize, i: usize) -> &EncInfo { &self.ei[i] }
    fn ei_mut(&mut self, _t: usize, i: usize) -> &mut EncInfo {
        &mut self.ei[i]
    }
}

trait TbEntry<T: TbTable> {
    fn table(&self) -> &T;
    fn table_mut(&self) -> &mut T;
}

trait EntryInfo {
    fn key(&self) -> u64;
    fn lock(&self) -> &Mutex<()>;
    fn num(&self) -> u8;
    fn symmetric(&self) -> bool;
    fn kk_enc(&self) -> bool;
    fn pawns(&self, i: usize) -> u8;
}

struct PieceEntry {
    key: u64,
    wdl: UnsafeCell<WdlPiece>,
    lock: Mutex<()>,
    num: u8,
    symmetric: bool,
    kk_enc: bool,
}

impl<T> TbEntry<T> for PieceEntry where T: TbTable {
    fn table_mut(&self) -> &mut T {
        unsafe { &mut *(self.wdl.get() as *mut T) }
    }

    fn table(&self) -> &T { self.table_mut() }
}

impl EntryInfo for PieceEntry {
    fn key(&self) -> u64 { self.key }
    fn lock(&self) -> &Mutex<()> { &self.lock }
    fn num(&self) -> u8 { self.num }
    fn symmetric(&self) -> bool { self.symmetric }
    fn kk_enc(&self) -> bool { self.kk_enc }
    fn pawns(&self, _i: usize) -> u8 { 0 }
}

struct WdlPawn {
    mapping: Option<Box<Mmap>>,
    ei: [[EncInfo; 2]; 4],
    ready: AtomicBool,
}

impl TbTable for WdlPawn {
    type Type = Wdl;
    type Entry = PawnEntry;
    type Enc = FileEnc;
    fn mapping(&mut self) -> &mut Option<Box<Mmap>> { &mut self.mapping }
    fn ready(&self) -> &AtomicBool { &self.ready }
    fn num_tables() -> usize { 4 }
    fn ei(&self, t: usize, i: usize) -> &EncInfo { &self.ei[t][i] }
    fn ei_mut(&mut self, t: usize, i: usize) -> &mut EncInfo {
        &mut self.ei[t][i]
    }
}

struct PawnEntry {
    key: u64,
    wdl: UnsafeCell<WdlPawn>,
    lock: Mutex<()>,
    num: u8,
    symmetric: bool,
    pawns: [u8; 2],
}

impl<T> TbEntry<T> for PawnEntry where T: TbTable {
    fn table_mut(&self) -> &mut T {
        unsafe { &mut *(self.wdl.get() as *mut T) }
    }

    fn table(&self) -> &T { self.table_mut() }
}

impl EntryInfo for PawnEntry {
    fn key(&self) -> u64 { self.key }
    fn lock(&self) -> &Mutex<()> { &self.lock }
    fn num(&self) -> u8 { self.num }
    fn symmetric(&self) -> bool { self.symmetric }
    fn kk_enc(&self) -> bool { false }
    fn pawns(&self, i: usize) -> u8 { self.pawns[i] }
}

#[derive(Clone, Debug)]
enum TbHashEntry {
    Piece(usize),
    Pawn(usize),
}

// Given a position with 6 or fewer pieces, produce a text string
// of the form KQPvKRP, where "KQP" represents the white pieces if
// flip == false and the black pieces if flip == true.
fn prt_str(board: &Board, flip: bool) -> String {
    const PIECE_TO_CHAR: [char; 6] = ['P', 'N', 'B', 'R', 'Q', 'K'];

    let mut c = if flip { Color::Black } else { Color::White };

    let mut s = String::new();

    for pt in (0..6).rev() {
        for _ in board.of(PieceType::new(pt), c) {
            s.push(PIECE_TO_CHAR[pt as usize]);
        }
    }
    s.push('v');
    c = !c;
    for pt in (0..6).rev() {
        for _ in board.of(PieceType::new(pt), c) {
            s.push(PIECE_TO_CHAR[pt as usize]);
        }
    }

    s
}

fn material(pc: usize, num: usize) -> u64 {
    unsafe { crate::types::zobrist::PSQ[pc][num] }
}

fn calc_key_from_pcs(pcs: &[i32; 16], flip: bool) -> u64 {
    let mut key = 0;

    for c in 0..2 {
        for pt in 1..7 {
            let pc = (c << 3) + pt as usize;
            for i in 0..pcs[pc as usize] {
                key ^= material(pc ^ ((flip as usize) << 3), i as usize);
            }
        }
    }

    key
}

fn calc_key_from_pieces(pieces: &[u8]) -> u64 {
    let mut key = 0;

    let mut cnt = [0; 16];

    for &k in pieces.iter() {
        let pc = k as usize;
        key ^= material(pc, cnt[k as usize]);
        cnt[k as usize] += 1;
    }

    key
}

static mut PATH: Option<String> = None;

fn sep_char() -> char {
    if cfg!(target_os = "windows") { ';' } else { ':' }
}

fn test_tb(name: &str, suffix: &str) -> bool {
    let dirs = unsafe { PATH.as_ref().unwrap().split(sep_char()) };
    for dir in dirs {
        let file_name = format!("{}{}{}{}", dir, '/', name, suffix);
        let path = Path::new(&file_name);
        if path.is_file() {
            return true;
        }
    }

    false
}

fn open_tb(name: &str, suffix: &str) -> Option<fs::File> {
    let dirs = unsafe { PATH.as_ref().unwrap().split(sep_char()) };
    for dir in dirs {
        let file_name = format!("{}{}{}{}", dir, '/', name, suffix);
        if let Ok(file) = fs::File::open(file_name) {
            return Some(file);
        }
    }

    None
}

fn map_file(name: &str, suffix: &str) -> Option<Box<Mmap>> {
    let file = open_tb(name, suffix);
    if file.is_none() {
        return None;
    }

    let file = file.unwrap();
    match unsafe { MmapOptions::new().map(&file) } {
        Ok(mmap) => {
            Some(Box::new(mmap))
        }
        Err(err) => {
            eprintln!("{:?}", err.kind());
            None
        }
    }
}

struct GlobalVec<T> {
    v: *mut T,
    cap: usize,
    len: usize,
}

impl<T> GlobalVec<T> {
    pub fn init(&mut self, cap: usize) {
        self.save(Vec::with_capacity(cap));
    }

    fn save(&mut self, mut v: Vec<T>) {
        self.v = v.as_mut_ptr();
        self.len = v.len();
        self.cap = v.capacity();
        std::mem::forget(v);
    }

    fn get_vec(&self) -> Vec<T> {
        unsafe { Vec::from_raw_parts(self.v, self.len, self.cap) }
    }

    pub fn push(&mut self, item: T) {
        let mut v = self.get_vec();
        v.push(item);
        self.save(v);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub unsafe fn reset(&mut self) {
        let mut v = self.get_vec();
        v.truncate(0);
        self.save(v);
    }

    pub unsafe fn free(&mut self) {
        std::mem::drop(self.get_vec());
    }
}

impl<T> std::ops::Index<usize> for GlobalVec<T> where T: 'static {
    type Output = T;

    fn index(&self, idx: usize) -> &'static T {
        unsafe {
            let elt_ref: &'static T = &*self.v.offset(idx as isize);
            elt_ref
        }
    }
}

static mut PIECE_ENTRIES: GlobalVec<PieceEntry> = GlobalVec { v: 0 as *mut PieceEntry, len: 0, cap: 0 };
static mut PAWN_ENTRIES: GlobalVec<PawnEntry> = GlobalVec { v: 0 as *mut PawnEntry, len: 0, cap: 0 };
static mut TB_MAP: *mut HashMap<u64, TbHashEntry> = 0 as *mut HashMap<u64, TbHashEntry>;

static mut NUM_WDL: u32 = 0;

pub fn init_tb(name: &str) {
    const PAWN  : usize = 1;
    const KNIGHT: usize = 2;
    const BISHOP: usize = 3;
    const ROOK  : usize = 4;
    const QUEEN : usize = 5;
    const KING  : usize = 6;

    const W_PAWN: usize = 1;
    const B_PAWN: usize = 9;

    if !test_tb(&name, WDL_SUFFIX) {
        return;
    }

    let mut pcs = [0; 16];
    let mut color = 0;
    for c in name.chars() {
        match c {
            'P' => pcs[PAWN   | color] += 1,
            'N' => pcs[KNIGHT | color] += 1,
            'B' => pcs[BISHOP | color] += 1,
            'R' => pcs[ROOK   | color] += 1,
            'Q' => pcs[QUEEN  | color] += 1,
            'K' => pcs[KING   | color] += 1,
            'v' => color = 8,
            _ => {}
        }
    }

    let key = calc_key_from_pcs(&pcs, false);
    let key2 = calc_key_from_pcs(&pcs, true);
    let symmetric = key == key2;

    let num = pcs.iter().sum::<i32>() as u32;
    unsafe {
        if num > MAX_CARDINALITY {
            MAX_CARDINALITY = num;
        }
    }

    let mut map = unsafe { Box::from_raw(TB_MAP) };

    let tb_entry;

    if pcs[W_PAWN] + pcs[B_PAWN] == 0 {
        let entry = PieceEntry {
            key: key,
            lock: Mutex::new(()),
            num: num as u8,
            symmetric: symmetric,
            kk_enc: pcs.iter().filter(|&n| *n == 1).count() == 2,
            wdl: UnsafeCell::new(WdlPiece {
                mapping: None,
                ready: AtomicBool::new(false),
                ei: [EncInfo::new(), EncInfo::new()],
            }),
        };
        unsafe { PIECE_ENTRIES.push(entry); }
        tb_entry = TbHashEntry::Piece(unsafe { PIECE_ENTRIES.len() - 1 });
    } else {
        let mut p0 = pcs[W_PAWN];
        let mut p1 = pcs[B_PAWN];
        if p1 > 0 && (p0 == 0 || p0 > p1) {
            std::mem::swap(&mut p0, &mut p1);
        }
        let entry = PawnEntry {
            key: key,
            lock: Mutex::new(()),
            num: num as u8,
            symmetric: symmetric,
            pawns: [p0 as u8, p1 as u8],
            wdl: UnsafeCell::new(WdlPawn {
                mapping: None,
                ready: AtomicBool::new(false),
                ei: [
                    [EncInfo::new(), EncInfo::new()],
                    [EncInfo::new(), EncInfo::new()],
                    [EncInfo::new(), EncInfo::new()],
                    [EncInfo::new(), EncInfo::new()],
                ],
            }),
        };
        unsafe { PAWN_ENTRIES.push(entry); }
        tb_entry = TbHashEntry::Pawn(unsafe { PAWN_ENTRIES.len() - 1 });
    }

    map.insert(key, tb_entry.clone());
    if key != key2 {
        map.insert(key2, tb_entry);
    }

    unsafe {
        TB_MAP = Box::into_raw(map);
        NUM_WDL += 1;
    }
}

pub fn free() {
    unsafe {
        std::mem::drop(Box::from_raw(TB_MAP));
        PIECE_ENTRIES.free();
        PAWN_ENTRIES.free();
    }
}

pub fn init(path: String) {
    const P: [char; 5] = [ 'Q', 'R', 'B', 'N', 'P' ];
    static mut INITIALIZED: bool = false;

    // Restrict engine to 5-piece TBs on platforms with 32-bit address space
    let max5 = std::mem::size_of::<usize>() < 8;

    unsafe {
        if !INITIALIZED {
            init_indices();
            PIECE_ENTRIES.init(if max5 { 84 } else { 254 });
            PAWN_ENTRIES.init(if max5 { 61 } else { 256 });
            TB_MAP = Box::into_raw(Box::new(HashMap::new()));
            INITIALIZED = true;
        }

        if PATH != None {
            PATH = None;
            std::mem::drop(Box::from_raw(TB_MAP));
            TB_MAP = Box::into_raw(Box::new(HashMap::new()));
            PIECE_ENTRIES.reset();
            PAWN_ENTRIES.reset();
            NUM_WDL = 0;
            MAX_CARDINALITY = 0;
        }
    }

    if path == "" || path == "<empty>" {
        return;
    }

    unsafe {
        PATH = Some(path);
    }

    for i in 0..5 {
        init_tb(&format!("K{}vK", P[i]));
    }

    for i in 0..5 {
        for j in i..5 {
            init_tb(&format!("K{}vK{}", P[i], P[j]));
        }
    }

    for i in 0..5 {
        for j in i..5 {
            init_tb(&format!("K{}{}vK", P[i], P[j]));
        }
    }

    for i in 0..5 {
        for j in i..5 {
            for k in 0..5 {
                init_tb(&format!("K{}{}vK{}", P[i], P[j], P[k]));
            }
        }
    }

    for i in 0..5 {
        for j in i..5 {
            for k in j..5 {
                init_tb(&format!("K{}{}{}vK", P[i], P[j], P[k]));
            }
        }
    }

    if !max5 {

        for i in 0..5 {
            for j in i..5 {
                for k in i..5 {
                    for l in (if i == k { j } else { k })..5 {
                        init_tb(&format!("K{}{}vK{}{}",
                            P[i], P[j], P[k], P[l]));
                    }
                }
            }
        }

        for i in 0..5 {
            for j in i..5 {
                for k in j..5 {
                    for l in 0..5 {
                        init_tb(&format!("K{}{}{}vK{}",
                            P[i], P[j], P[k], P[l]));
                    }
                }
            }
        }

        for i in 0..5 {
            for j in i..5 {
                for k in j..5 {
                    for l in k..5 {
                        init_tb(&format!("K{}{}{}{}vK",
                            P[i], P[j], P[k], P[l]));
                    }
                }
            }
        }

        for i in 0..5 {
            for j in i..5 {
                for k in j..5 {
                    for l in 0..5 {
                        for m in l..5 {
                            init_tb(&format!("K{}{}{}vK{}{}",
                                P[i], P[j], P[k], P[l], P[m]));
                        }
                    }
                }
            }
        }

        for i in 0..5 {
            for j in i..5 {
                for k in j..5 {
                    for l in k..5 {
                        for m in 0..5 {
                            init_tb(&format!("K{}{}{}{}vK{}",
                                P[i], P[j], P[k], P[l], P[m]));
                        }
                    }
                }
            }
        }

        for i in 0..5 {
            for j in i..5 {
                for k in j..5 {
                    for l in k..5 {
                        for m in l..5 {
                            init_tb(&format!("K{}{}{}{}{}vK",
                                P[i], P[j], P[k], P[l], P[m]));
                        }
                    }
                }
            }
        }

    }

    println!("info string Found {} WDL tablebase files.", unsafe { NUM_WDL });
}

// place k like pieces on n squares
fn subfactor(k: usize, n: usize) -> usize {
    let mut f = n;
    let mut l = 1;
    for i in 1..k {
        f *= n - i;
        l *= i + 1;
    }

    f / l
}

fn calc_factors<T: Encoding>(ei: &mut EncInfo, e: &T::Entry, order: u8, order2: u8, t: usize) -> usize {
    let mut i = ei.norm[0];
    if order2 < 0x0f {
        i += ei.norm[i as usize];
    }
    let mut n = 64 - i;
    let mut f = 1;
    let mut k = 0;
    while i < e.num() || k == order || k == order2 {
        if k == order {
            ei.factor[0] = f;
            f *= if T::ENC == PieceEnc::ENC {
                if e.kk_enc() { 462 } else { 31332 }
            } else {
                pfactor::<T>(ei.norm[0] as usize - 1, t)
            };
        } else if k == order2 {
            ei.factor[ei.norm[0] as usize] = f;
            f *= subfactor(ei.norm[ei.norm[0] as usize] as usize,
                48 - ei.norm[0] as usize);
        } else {
            ei.factor[i as usize] = f;
            f *= subfactor(ei.norm[i as usize] as usize, n as usize);
            n -= ei.norm[i as usize];
            i += ei.norm[i as usize];
        }
        k += 1;
    }

    f
}

fn set_norm<T: Encoding>(ei: &mut EncInfo, e: &T::Entry) {
    let mut i;
    if T::ENC == PieceEnc::ENC {
        ei.norm[0] = if e.kk_enc() { 2 } else { 3 };
        i = ei.norm[0] as usize;
    } else {
        ei.norm[0] = e.pawns(0);
        if e.pawns(1) > 0 {
            ei.norm[e.pawns(0) as usize] = e.pawns(1);
        }
        i = (e.pawns(0) + e.pawns(1)) as usize;
    }

    while i < e.num() as usize {
        for j in i..e.num() as usize {
            if ei.pieces[j] != ei.pieces[i] {
                break;
            }
            ei.norm[i] += 1;
        }
        i += ei.norm[i] as usize;
    }
}

fn setup_pieces<T: Encoding>(ei: &mut EncInfo, e: &T::Entry, tb: &[u8], s: u32, t: usize) -> usize {
    let j = 1 + (e.pawns(1) > 0) as usize;

    for i in 0..(e.num() as usize) {
        ei.pieces[i] = (tb[i + j] >> s) & 0x0f;
    }
    let order = (tb[0] >> s) & 0x0f;
    let order2 =
        if e.pawns(1) > 0 { (tb[1] >> s) & 0x0f } else { 0x0f };

    set_norm::<T>(ei, e);
    calc_factors::<T>(ei, e, order, order2, t)
}

#[repr(packed)]
struct IndexEntry {
    block: u32,
    offset: u16,
}

struct PairsData {
    index_table: &'static [IndexEntry],
    size_table: &'static [u16],
    data: &'static [u8],
    offset: &'static [u16],
    sym_len: Vec<u8>,
    sym_pat: &'static [[u8; 3]],
    block_size: u32,
    idx_bits: u32,
    min_len: u8,
    const_val: u16,
    base: Vec<u64>,
}

fn s1(w: &[u8; 3]) -> usize {
    (w[0] as usize) | ((w[1] as usize & 0x0f) << 8)
}

fn s2(w: &[u8; 3]) -> usize {
    ((w[2] as usize) << 4) | ((w[1] as usize) >> 4)
}

fn calc_sym_len(sym_len: &mut Vec<u8>, sym_pat: &[[u8; 3]], s: usize, tmp: &mut Vec<u8>) {
    if tmp[s] != 0 {
        return;
    }

    let w = &sym_pat[s];
    let s2 = s2(w);
    if s2 == 0x0fff {
        sym_len[s] = 0;
    } else {
        let s1 = s1(w);
        calc_sym_len(sym_len, sym_pat, s1, tmp);
        calc_sym_len(sym_len, sym_pat, s2, tmp);
        sym_len[s] = sym_len[s1] + sym_len[s2] + 1;
    }
    tmp[s] = 1;
}

fn setup_pairs(data_ref: &mut &'static [u8], tb_size: usize, size: &mut [usize], flags: &mut u8, is_wdl: bool) -> Box<PairsData> {
    let data = *data_ref;
    *flags = data[0];
    if *flags & 0x80 != 0 {
        *data_ref = &data[2..];
        return Box::new(PairsData {
            index_table: &[],
            size_table: &[],
            data: &[],
            offset: &[],
            sym_len: Vec::new(),
            sym_pat: &[],
            block_size: 0,
            idx_bits: 0,
            min_len: 0,
            const_val: if is_wdl { data[1] as u16 } else { 0 },
            base: Vec::new(),
        });
    }

    let block_size = data[1] as u32;
    let idx_bits = data[2] as u32;
    let real_num_blocks = u32::from_le(cast_slice(&data[4..], 1)[0]);
    let num_blocks = real_num_blocks + data[3] as u32;
    let max_len = data[8];
    let min_len = data[9];
    let h = (max_len - min_len + 1) as usize;
    let num_syms = u16::from_le(cast_slice(&data[10 + 2 * h..], 1)[0]) as usize;
    let mut sym_len = Vec::with_capacity(num_syms);
    for _ in 0..num_syms {
        sym_len.push(0u8);
    }
    let sym_pat = cast_slice::<[u8; 3]>(&data[12 + 2 * h..], num_syms);

    let mut tmp = Vec::with_capacity(num_syms);
    for _ in 0..num_syms {
        tmp.push(0u8);
    }
    for s in 0..num_syms {
        calc_sym_len(&mut sym_len, sym_pat, s, &mut tmp);
    }

    let num_indices = (tb_size + (1usize << idx_bits) - 1) >> idx_bits;
    size[0] = num_indices as usize;
    size[1] = num_blocks as usize;
    size[2] = (real_num_blocks as usize) << block_size;

    *data_ref = &data[12 + 2 * h + 3 * num_syms + (num_syms & 1)..];

    let offset = cast_slice::<u16>(&data[10..], h);
    let mut base = Vec::with_capacity(h);
    for _ in 0..h {
        base.push(0u64);
    }
    for i in (0..h-1).rev() {
        let b1 = u16::from_le(offset[i]) as u64;
        let b2 = u16::from_le(offset[i + 1]) as u64;
        base[i] = (base[i + 1] + b1 - b2) / 2;
    }
    for i in 0..h {
        base[i] <<= 64 - (min_len as usize + i);
    }

    Box::new(PairsData {
        index_table: &[],
        size_table: &[],
        data: &[],
        offset: offset,
        sym_len: sym_len,
        sym_pat: sym_pat,
        block_size: block_size,
        idx_bits: idx_bits,
        min_len: min_len,
        const_val: 0,
        base: base,
    })
}

fn align_slice(data: &[u8], align: usize) -> &[u8] {
    let ptr1 = data.as_ptr() as usize;
    let ptr2 = (ptr1 + align - 1) & !(align - 1);
    &data[(ptr2 - ptr1)..]
}

fn slice<'a, T>(data: &mut &'a [u8], size: usize) -> &'a [T] {
    let ptr = data.as_ptr();
    *data = &data[size * std::mem::size_of::<T>()..];
    unsafe {
        slice::from_raw_parts(ptr as *const T, size)
    }
}

fn cast_slice<T>(data: &[u8], size: usize) -> &[T] {
    assert!(data.len() >= size * std::mem::size_of::<T>());

    unsafe {
        slice::from_raw_parts(data.as_ptr() as *const T, size)
    }
}

fn read_magic(mmap: &Option<Box<Mmap>>) -> u32 {
    let data: &[u8] = &*mmap.as_ref().unwrap();
    u32::from_le(cast_slice(data, 1)[0])
}

fn mmap_to_slice(mmap: &Option<Box<Mmap>>) -> &'static [u8] {
    let data: &[u8] = &*mmap.as_ref().unwrap();
    unsafe {
        slice::from_raw_parts(data.as_ptr(), data.len())
    }
}

fn init_table<T: TbTable>(e: &T::Entry, name: &str) -> bool {
    let tb_map = map_file(name, T::Type::suffix());
    if tb_map.is_none() {
        return false;
    }

    if read_magic(&tb_map) != T::Type::magic() {
        eprintln!("Corrupted table: {}{}", name, T::Type::suffix());
        return false;
    }

    let mut tb = e.table_mut();
    *tb.mapping() = tb_map;
    let mut data = mmap_to_slice(tb.mapping());

    let split = data[4] & 0x01 != 0;

    data = &data[5..];
    let mut tb_size = [[0; 2]; 6];
    let num = T::num_tables();
    for t in 0..num {
        tb_size[t][0] =
            setup_pieces::<T::Enc>(tb.ei_mut(t, 0), e, data, 0, t);
        if split {
            tb_size[t][1] =
                setup_pieces::<T::Enc>(tb.ei_mut(t, 1), e, data, 4, t);
        }
        data = &data[e.num() as usize + 1 + (e.pawns(1) > 0) as usize..];
    }
    data = align_slice(data, 2);

    let mut size = [[0; 6]; 6];
    let mut flags = 0;
    for t in 0..num {
        tb.ei_mut(t, 0).precomp = Some(setup_pairs(&mut data, tb_size[t][0],
            &mut size[t][0..3], &mut flags, true));
        if split {
            tb.ei_mut(t, 1).precomp = Some(setup_pairs(&mut data,
                tb_size[t][1], &mut size[t][3..6], &mut flags, true));
        }
    }

    for t in 0..num {
        tb.ei_mut(t, 0).precomp.as_mut().unwrap().index_table =
            slice(&mut data, size[t][0]);
        if split {
            tb.ei_mut(t, 1).precomp.as_mut().unwrap().index_table =
                slice(&mut data, size[t][3]);
        }
    }

    for t in 0..num {
        tb.ei_mut(t, 0).precomp.as_mut().unwrap().size_table =
            slice(&mut data, size[t][1]);
        if split {
            tb.ei_mut(t, 1).precomp.as_mut().unwrap().size_table =
                slice(&mut data, size[t][4]);
        }
    }

    for t in 0..num {
        data = align_slice(data, 64);
        tb.ei_mut(t, 0).precomp.as_mut().unwrap().data =
            slice(&mut data, size[t][2]);
        if split {
            data = align_slice(data, 64);
            tb.ei_mut(t, 1).precomp.as_mut().unwrap().data =
                slice(&mut data, size[t][5]);
        }
    }

    true
}

fn fill_squares(board: &Board, pc: &[u8; TB_PIECES], num: usize, flip: bool, p: &mut [Square; TB_PIECES]) {
    const MAP: [usize; 7] = [6, 0, 1, 2, 3, 4, 5];

    let mut i = 0;
    loop {
        let piece = pc[i] as u32;
        let color = (piece >> 3) as usize ^ flip as usize;

        let b = board.of(PieceType::new(MAP[piece as usize & 7]), if color == 0 { Color::White} else { Color::Black });
        for sq in b {
            p[i] = sq;
            i += 1;
        }
        if i == num as usize {
            break;
        }
    }
}

fn probe_helper<T: TbTable> (board: &Board, e: &T::Entry, success: &mut i32) -> i32 {
    let key = board.material_key();

    let tb = e.table();
    if !tb.ready().load(Ordering::Acquire) {
        let _lock = e.lock().lock().unwrap();
        if !tb.ready().load(Ordering::Relaxed) {
            if !init_table::<T>(e, &prt_str(board, e.key() != key)) {
                *success = 0;
                return 0;
            }
            tb.ready().store(true, Ordering::Release);
        }
    }

    let flip = if !e.symmetric() { (key != e.key()) != false }
        else { board.side_to_move() != Color::White };
    let bside = (!e.symmetric()
        && (((key != e.key()) != false) ==
            (board.side_to_move() == Color::White))) as usize;

    let t = if T::Enc::ENC != PieceEnc::ENC {
        let color = (tb.ei(0, 0).pieces[0] as u32) >> 3;
        let b = board.of(PieceType::Pawn, if color ^ flip as u32 == 0 { Color::White } else { Color::Black });
        leading_pawn_table::<T::Enc>(b, flip) as usize
    } else { 0 };

    let mut p: [Square; TB_PIECES] = [Square::new(0); TB_PIECES];
    fill_squares(board, &tb.ei(t, bside).pieces, e.num() as usize, flip,
            &mut p);
    if T::Enc::ENC != PieceEnc::ENC && flip {
        for i in 0..e.num() as usize {
            p[i] = Square::new((p[i] as u8) ^ (Square::A8 as u8));
        }
    }
    let idx = encode::<T::Enc>(&mut p, &tb.ei(t, bside), e);

    let res = decompress_pairs(&tb.ei(t, bside).precomp.as_ref().unwrap(), idx);

    res - 2
}

fn probe_table<T: TbType>(board: &Board, success: &mut i32) -> i32 {
    // Obtain the position's material signature key
    let key = board.material_key();

    // Test for KvK
    if board.occupancies() == board.pieces(PieceType::King) {
        return 0;
    }

    let mut res = 0;
    let map = unsafe { Box::from_raw(TB_MAP) };

    match map.get(&key) {
        None => {
            *success = 0;
        }
        Some(&TbHashEntry::Piece(idx)) => {
            let e = unsafe { &PIECE_ENTRIES[idx] };
            res = probe_helper::<T::PieceTable>(board, e, success);
        }
        Some(&TbHashEntry::Pawn(idx)) => {
            let e = unsafe { &PAWN_ENTRIES[idx] };
            res = probe_helper::<T::PawnTable>(board, e, success);
        }
    }

    std::mem::forget(map);

    res
}

fn probe_ab(board: &mut Board, mut alpha: i32, beta: i32, success: &mut i32) -> i32 {
    let mut list = MoveList::new();

    if board.checkers().is_empty() {
        board.append_noisy_moves(&mut list);
    } else {
        board.append_all_moves(&mut list);
    }

    for &entry in list.iter() {
        if !entry.mv.is_capture() || !board.is_legal(entry.mv) {
            continue;
        }

        board.make_move(entry.mv);
        let v = -probe_ab(board, -beta, -alpha, success);
        board.undo_move(entry.mv);

        if *success == 0 {
            return 0;
        }

        if v > alpha {
            if v >= beta {
                return v;
            }
            alpha = v;
        }
    }

    let v = probe_table::<Wdl>(board, success);

    if alpha >= v { alpha } else { v }
}

// Probe the WDL table for a particular position.
//
// If *success != 0, the probe was successful.
//
// If *success == 2, the position has a winning capture, or the position
// is a cursed win and has a cursed winning capture, or the positoin has
// en ep captures as only best move.
// This information is used in probe_dtz().
//
// The return value is from the point of view of the side to move.
// -2 : loss
// -1 : loss, but draw under the 50-move rule
//  0 : draw
//  1 : win, but draw under the 50-move rule
//  2 : win
pub fn probe_wdl(board: &mut Board, success: &mut i32) -> i32 {
    // Generate (at least) all legal en-passant captures
    let mut list = MoveList::new();

    if board.checkers().is_empty() {
        board.append_noisy_moves(&mut list);
    } else {
        board.append_all_moves(&mut list);
    }

    let mut best_cap = -3;
    let mut best_ep = -3;

    for &entry in list.iter() {
        if !entry.mv.is_capture() || !board.is_legal(entry.mv) {
            continue;
        }

        board.make_move(entry.mv);
        let v = -probe_ab(board, -2, -best_cap, success);
        board.undo_move(entry.mv);

        if *success == 0 {
            return 0;
        }

        if v > best_cap {
            if v == 2 {
                *success = 2;
                return 2;
            }

            if !entry.mv.is_en_passant() {
                best_cap = v;
            } else if v > best_ep {
                best_ep = v;
            }
        }
    }

    let v = probe_table::<Wdl>(board, success);
    if *success == 0 {
        return 0;
    }

    // Now max(v, best_cap) is the WDL value of the position wihtout
    // ep rights. If the position without ep rights is not stalemate or
    // no ep captures exist, then the value of the position is
    // max(v, best_cap, best_ep). If the position without ep rights is
    // stalemate and best_ep > -3, then the value of the position is
    // best_ep (and we will have v == 0).

    if best_ep > best_cap {
        if best_ep > v {
            // ep capture (possibly cursed losing) is best
            *success = 2;
            return best_ep;
        }
        best_cap = best_ep;
    }

    // Now max(v, best_cap) is the WDL value of the position, unless the
    // position without ep rights is stalemate and best_ep > -3.

    if best_cap >= v {
        // No need to test for the stalemate case here: either there are
        // non-ep captures, or best_cap == best_ep >= v anyway.
        *success = 1 + (best_cap > 0) as i32;
        return best_cap;
    }

    // Now handle the stalemate case.
    if best_ep > -3 && v == 0 {
        // Check for stalemate in the position without ep captures.
        for &entry in list.iter() {
            if !entry.mv.is_en_passant() && board.is_legal(entry.mv) {
                return v;
            }
        }
        if board.checkers().is_empty() {
            board.append_quiet_moves(&mut list);
            for &entry in list.iter() {
                if !entry.mv.is_en_passant() && board.is_legal(entry.mv) {
                    return v;
                }
            }
        }
        *success = 2;
        return best_ep;
    }

    v
}

const OFF_DIAG: [i8; 64] = [
    0, -1, -1, -1, -1, -1, -1, -1,
    1,  0, -1, -1, -1, -1, -1, -1,
    1,  1,  0, -1, -1, -1, -1, -1,
    1,  1,  1,  0, -1, -1, -1, -1,
    1,  1,  1,  1,  0, -1, -1, -1,
    1,  1,  1,  1,  1,  0, -1, -1,
    1,  1,  1,  1,  1,  1,  0, -1,
    1,  1,  1,  1,  1,  1,  1,  0,
];

const TRIANGLE: [u8; 64] = [
    6, 0, 1, 2, 2, 1, 0, 6,
    0, 7, 3, 4, 4, 3, 7, 0,
    1, 3, 8, 5, 5, 8, 3, 1,
    2, 4, 5, 9, 9, 5, 4, 2,
    2, 4, 5, 9, 9, 5, 4, 2,
    1, 3, 8, 5, 5, 8, 3, 1,
    0, 7, 3, 4, 4, 3, 7, 0,
    6, 0, 1, 2, 2, 1, 0, 6,
];

const FLIP_DIAG: [u8; 64] = [
    0,  8, 16, 24, 32, 40, 48, 56,
    1,  9, 17, 25, 33, 41, 49, 57,
    2, 10, 18, 26, 34, 42, 50, 58,
    3, 11, 19, 27, 35, 43, 51, 59,
    4, 12, 20, 28, 36, 44, 52, 60,
    5, 13, 21, 29, 37, 45, 53, 61,
    6, 14, 22, 30, 38, 46, 54, 62,
    7, 15, 23, 31, 39, 47, 55, 63,
];

const LOWER: [u8; 64] = [
    28,  0,  1,  2,  3,  4,  5,  6,
     0, 29,  7,  8,  9, 10, 11, 12,
     1,  7, 30, 13, 14, 15, 16, 17,
     2,  8, 13, 31, 18, 19, 20, 21,
     3,  9, 14, 18, 32, 22, 23, 24,
     4, 10, 15, 19, 22, 33, 25, 26,
     5, 11, 16, 20, 23, 25, 34, 27,
     6, 12, 17, 21, 24, 26, 27, 35,
];

const DIAG: [u8; 64] = [
     0,  0,  0,  0,  0,  0,  0,  8,
     0,  1,  0,  0,  0,  0,  9,  0,
     0,  0,  2,  0,  0, 10,  0,  0,
     0,  0,  0,  3, 11,  0,  0,  0,
     0,  0,  0, 12,  4,  0,  0,  0,
     0,  0, 13,  0,  0,  5,  0,  0,
     0, 14,  0,  0,  0,  0,  6,  0,
    15,  0,  0,  0,  0,  0,  0,  7,
];

const FLAP: [u8; 64] = [
    0,  0,  0,  0,  0,  0,  0, 0,
    0,  6, 12, 18, 18, 12,  6, 0,
    1,  7, 13, 19, 19, 13,  7, 1,
    2,  8, 14, 20, 20, 14,  8, 2,
    3,  9, 15, 21, 21, 15,  9, 3,
    4, 10, 16, 22, 22, 16, 10, 4,
    5, 11, 17, 23, 23, 17, 11, 5,
    0,  0,  0,  0,  0,  0,  0, 0,
];

const PTWIST: [u8; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    47, 35, 23, 11, 10, 22, 34, 46,
    45, 33, 21,  9,  8, 20, 32, 44,
    43, 31, 19,  7,  6, 18, 30, 42,
    41, 29, 17,  5,  4, 16, 28, 40,
    39, 27, 15,  3,  2, 14, 26, 38,
    37, 25, 13,  1,  0, 12, 24, 36,
     0,  0,  0,  0,  0,  0,  0,  0
];

const FLAP2: [u8; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
     0,  1,  2,  3,  3,  2,  1,  0,
     4,  5,  6,  7,  7,  6,  5,  4,
     8,  9, 10, 11, 11, 10,  9,  8,
    12, 13, 14, 15, 15, 14, 13, 12,
    16, 17, 18, 19, 19, 18, 17, 16,
    20, 21, 22, 23, 23, 22, 21, 20,
     0,  0,  0,  0,  0,  0,  0,  0,
];

const PTWIST2: [u8; 64] = [
     0,  0,  0,  0,  0,  0,  0,  0,
    47, 45, 43, 41, 40, 42, 44, 46,
    39, 37, 35, 33, 32, 34, 36, 38,
    31, 29, 27, 25, 24, 26, 28, 30,
    23, 21, 19, 17, 16, 18, 20, 22,
    15, 13, 11,  9,  8, 10, 12, 14,
     7,  5,  3,  1,  0,  2,  4,  6,
     0,  0,  0,  0,  0,  0,  0,  0,
];

const KK_IDX: [[u16; 64]; 10] = [
    [   0,   0,   0,   0,   1,   2,   3,   4,
        0,   0,   0,   5,   6,   7,   8,   9,
       10,  11,  12,  13,  14,  15,  16,  17,
       18,  19,  20,  21,  22,  23,  24,  25,
       26,  27,  28,  29,  30,  31,  32,  33,
       34,  35,  36,  37,  38,  39,  40,  41,
       42,  43,  44,  45,  46,  47,  48,  49,
       50,  51,  52,  53,  54,  55,  56,  57, ],
    [  58,   0,   0,   0,  59,  60,  61,  62,
       63,   0,   0,   0,  64,  65,  66,  67,
       68,  69,  70,  71,  72,  73,  74,  75,
       76,  77,  78,  79,  80,  81,  82,  83,
       84,  85,  86,  87,  88,  89,  90,  91,
       92,  93,  94,  95,  96,  97,  98,  99,
      100, 101, 102, 103, 104, 105, 106, 107,
      108, 109, 110, 111, 112, 113, 114, 115 ],
    [ 116, 117,   0,   0,   0, 118, 119, 120,
      121, 122,   0,   0,   0, 123, 124, 125,
      126, 127, 128, 129, 130, 131, 132, 133,
      134, 135, 136, 137, 138, 139, 140, 141,
      142, 143, 144, 145, 146, 147, 148, 149,
      150, 151, 152, 153, 154, 155, 156, 157,
      158, 159, 160, 161, 162, 163, 164, 165,
      166, 167, 168, 169, 170, 171, 172, 173 ],
    [ 174,   0,   0,   0, 175, 176, 177, 178,
      179,   0,   0,   0, 180, 181, 182, 183,
      184,   0,   0,   0, 185, 186, 187, 188,
      189, 190, 191, 192, 193, 194, 195, 196,
      197, 198, 199, 200, 201, 202, 203, 204,
      205, 206, 207, 208, 209, 210, 211, 212,
      213, 214, 215, 216, 217, 218, 219, 220,
      221, 222, 223, 224, 225, 226, 227, 228 ],
    [ 229, 230,   0,   0,   0, 231, 232, 233,
      234, 235,   0,   0,   0, 236, 237, 238,
      239, 240,   0,   0,   0, 241, 242, 243,
      244, 245, 246, 247, 248, 249, 250, 251,
      252, 253, 254, 255, 256, 257, 258, 259,
      260, 261, 262, 263, 264, 265, 266, 267,
      268, 269, 270, 271, 272, 273, 274, 275,
      276, 277, 278, 279, 280, 281, 282, 283 ],
    [ 284, 285, 286, 287, 288, 289, 290, 291,
      292, 293,   0,   0,   0, 294, 295, 296,
      297, 298,   0,   0,   0, 299, 300, 301,
      302, 303,   0,   0,   0, 304, 305, 306,
      307, 308, 309, 310, 311, 312, 313, 314,
      315, 316, 317, 318, 319, 320, 321, 322,
      323, 324, 325, 326, 327, 328, 329, 330,
      331, 332, 333, 334, 335, 336, 337, 338 ],
    [   0,   0, 339, 340, 341, 342, 343, 344,
        0,   0, 345, 346, 347, 348, 349, 350,
        0,   0, 441, 351, 352, 353, 354, 355,
        0,   0,   0, 442, 356, 357, 358, 359,
        0,   0,   0,   0, 443, 360, 361, 362,
        0,   0,   0,   0,   0, 444, 363, 364,
        0,   0,   0,   0,   0,   0, 445, 365,
        0,   0,   0,   0,   0,   0,   0, 446 ],
    [   0,   0,   0, 366, 367, 368, 369, 370,
        0,   0,   0, 371, 372, 373, 374, 375,
        0,   0,   0, 376, 377, 378, 379, 380,
        0,   0,   0, 447, 381, 382, 383, 384,
        0,   0,   0,   0, 448, 385, 386, 387,
        0,   0,   0,   0,   0, 449, 388, 389,
        0,   0,   0,   0,   0,   0, 450, 390,
        0,   0,   0,   0,   0,   0,   0, 451 ],
    [ 452, 391, 392, 393, 394, 395, 396, 397,
        0,   0,   0,   0, 398, 399, 400, 401,
        0,   0,   0,   0, 402, 403, 404, 405,
        0,   0,   0,   0, 406, 407, 408, 409,
        0,   0,   0,   0, 453, 410, 411, 412,
        0,   0,   0,   0,   0, 454, 413, 414,
        0,   0,   0,   0,   0,   0, 455, 415,
        0,   0,   0,   0,   0,   0,   0, 456 ],
    [ 457, 416, 417, 418, 419, 420, 421, 422,
        0, 458, 423, 424, 425, 426, 427, 428,
        0,   0,   0,   0,   0, 429, 430, 431,
        0,   0,   0,   0,   0, 432, 433, 434,
        0,   0,   0,   0,   0, 435, 436, 437,
        0,   0,   0,   0,   0, 459, 438, 439,
        0,   0,   0,   0,   0,   0, 460, 440,
        0,   0,   0,   0,   0,   0,   0, 461 ],
];

static mut BINOMIAL: [[usize; 64]; 7] = [[0; 64]; 7];
static mut PAWN_IDX: [[usize; 24]; 6] = [[0; 24]; 6];
static mut PFACTOR: [[usize; 4]; 6] = [[0; 4]; 6];
static mut PAWN_IDX2: [[usize; 24]; 6] = [[0; 24]; 6];
static mut PFACTOR2: [[usize; 6]; 6] = [[0; 6]; 6];

fn off_diag(s: Square) -> i8 {
    OFF_DIAG[s as usize]
}

fn is_off_diag(s: Square) -> bool {
    off_diag(s) != 0
}

fn triangle(s: Square) -> usize {
    TRIANGLE[s as usize] as usize
}

fn flip_diag(s: Square) -> Square {
    Square::new(FLIP_DIAG[s as usize] as u8)
}

fn lower(s: Square) -> usize {
    LOWER[s as usize] as usize
}

fn diag(s: Square) -> usize {
    DIAG[s as usize] as usize
}

fn skip(s1: Square, s2: Square) -> usize {
    (s1 as u8 > s2 as u8) as usize
}

fn flap<T: Encoding>(s: Square) -> usize {
    if T::ENC == FileEnc::ENC {
        FLAP[s as usize] as usize
    } else {
        FLAP2[s as usize] as usize
    }
}

fn ptwist<T: Encoding>(s: Square) -> usize {
    if T::ENC == FileEnc::ENC {
        PTWIST[s as usize] as usize
    } else {
        PTWIST2[s as usize] as usize
    }
}

fn kk_idx(s1: usize, s2: Square) -> usize {
    KK_IDX[s1][s2 as usize] as usize
}

fn binomial(n: usize, k: usize) -> usize {
    unsafe { BINOMIAL[k as usize][n] }
}

fn pawn_idx<T: Encoding>(num: usize, s: usize) -> usize {
    if T::ENC == FileEnc::ENC {
        unsafe { PAWN_IDX[num][s] }
    } else {
        unsafe { PAWN_IDX2[num][s] }
    }
}

fn pfactor<T: Encoding>(num: usize, s: usize) -> usize {
    if T::ENC == FileEnc::ENC {
        unsafe { PFACTOR[num][s] }
    } else {
        unsafe { PFACTOR2[num][s] }
    }
}

fn init_indices() {
    for i in 0..7 {
        for j in 0..64 {
            let mut f = 1;
            let mut l = 1;
            for k in 0..i {
                f *= usize::wrapping_sub(j, k);
                l *= k + 1;
            }
            unsafe { BINOMIAL[i][j] = f / l; }
        }
    }

    for i in 0..6 {
        let mut s = 0;
        for j in 0..24 {
            unsafe { PAWN_IDX[i][j] = s; }
            let k = (1 + (j % 6)) * 8 + (j / 6);
            s += binomial(ptwist::<FileEnc>(Square::new(k as u8)), i);
            if (j + 1) % 6 == 0 {
                unsafe { PFACTOR[i][j / 6] = s; }
                s = 0;
            }
        }
    }

    for i in 0..6 {
        let mut s = 0;
        for j in 0..24 {
            unsafe { PAWN_IDX2[i][j] = s; }
            let k = (1 + (j / 4)) * 8 + (j % 4);
            s += binomial(ptwist::<RankEnc>(Square::new(k as u8)), i);
            if (j + 1) % 4 == 0 {
                unsafe { PFACTOR2[i][j / 4] = s; }
                s = 0;
            }
        }
    }
}

pub const FILE_A: u32 = 0;
pub const FILE_B: u32 = 1;
pub const FILE_C: u32 = 2;
pub const FILE_D: u32 = 3;
pub const FILE_E: u32 = 4;
pub const FILE_F: u32 = 5;
pub const FILE_G: u32 = 6;
pub const FILE_H: u32 = 7;

pub const FILEA_BB: Bitboard = Bitboard(0x0101010101010101);
pub const FILEB_BB: Bitboard = Bitboard(0x0202020202020202);
pub const FILEC_BB: Bitboard = Bitboard(0x0404040404040404);
pub const FILED_BB: Bitboard = Bitboard(0x0808080808080808);
pub const FILEE_BB: Bitboard = Bitboard(0x1010101010101010);
pub const FILEF_BB: Bitboard = Bitboard(0x2020202020202020);
pub const FILEG_BB: Bitboard = Bitboard(0x4040404040404040);
pub const FILEH_BB: Bitboard = Bitboard(0x8080808080808080);

fn leading_pawn_table<T: Encoding>(pawns: Bitboard, flip: bool) -> u32 {
    if T::ENC == FileEnc::ENC {
        if pawns & (FILEA_BB | FILEB_BB | FILEG_BB | FILEH_BB) != Bitboard(0) {
            if pawns & (FILEA_BB | FILEH_BB) != Bitboard(0) { FILE_A } else { FILE_B }
        } else {
            if pawns & (FILEC_BB | FILEF_BB) != Bitboard(0) { FILE_C } else { FILE_D }
        }
    } else {
        let b = if flip { Bitboard(pawns.0.swap_bytes()) } else { pawns };
        b.lsb().rank() as u32 - 1
    }
}

fn encode<T: Encoding>(p: &mut [Square; TB_PIECES], ei: &EncInfo, entry: &T::Entry) -> usize {
    let n = entry.num() as usize;

    if T::ENC != PieceEnc::ENC {
        for i in 0..entry.pawns(0) {
            for j in i+1..entry.pawns(0) {
                if ptwist::<T>(p[i as usize]) < ptwist::<T>(p[j as usize])
                {
                    p.swap(i as usize, j as usize);
                }
            }
        }
    }

    if p[0] as usize & 0x04 != 0 {
        for i in 0..n {
            p[i] = Square::new(p[i] as u8 ^ 0x07);
        }
    }

    let mut i;
    let mut idx;
    if T::ENC == PieceEnc::ENC {
        if p[0] as usize & 0x20 != 0 {
            for i in 0..n {
                p[i] = Square::new(p[i] as u8 ^ 0x38);
            }
        }

        for i in 0..n {
            if is_off_diag(p[i]) {
                if off_diag(p[i]) > 0
                    && i < (if entry.kk_enc() { 2 } else { 3 })
                {
                    for j in i..n {
                        p[j] = flip_diag(p[j]);
                    }
                }
                break;
            }
        }

        idx = if entry.kk_enc() {
            i = 2;
            kk_idx(triangle(p[0]), p[1])
        } else {
            i = 3;
            let s1 = skip(p[1], p[0]);
            let s2 = skip(p[2], p[0]) + skip(p[2], p[1]);
            if is_off_diag(p[0]) {
                triangle(p[0]) * 63*62 + (p[1] as usize - s1) * 62
                + (p[2] as usize - s2)
            } else if is_off_diag(p[1]) {
                6*63*62 + diag(p[0]) * 28*62 + lower(p[1]) * 62
                + p[2] as usize - s2
            } else if is_off_diag(p[2]) {
                6*63*62 + 4*28*62 + diag(p[0]) * 7*28
                + (diag(p[1]) - s1) * 28 + lower(p[2])
            } else {
                6*63*62 + 4*28*62 + 4*7*28 + diag(p[0]) * 7*6
                + (diag(p[1]) - s1) * 6 + (diag(p[2]) - s2)
            }
        };
        idx *= ei.factor[0];
    } else {
        let t = entry.pawns(0) as usize;
        idx = pawn_idx::<T>(t - 1, flap::<T>(p[0])) as usize;
        for i in 1..t {
            idx += binomial(ptwist::<T>(p[i]), t - i);
        }
        idx *= ei.factor[0];

        // remaining pawns
        i = entry.pawns(0) as usize;
        let t = i + entry.pawns(1) as usize;
        if t > i {
            for j in i..t {
                for k in j+1..t {
                    if p[j] as usize > p[k] as usize {
                        p.swap(j, k);
                    }
                }
            }
            let mut s = 0;
            for m in i..t {
                let sq = p[m];
                let mut skips = 0;
                for k in 0..i {
                    skips += skip(sq, p[k]);
                }
                s += binomial(sq as usize - skips - 8, m - i + 1);
            }
            idx += s * ei.factor[i];
            i = t;
        }
    }

    while i < n {
        let t = ei.norm[i] as usize;
        for j in i..i+t {
            for k in j+1..i+t {
                if p[j] as usize > p[k] as usize {
                    p.swap(j, k);
                }
            }
        }
        let mut s = 0;
        for m in i..i+t {
            let sq = p[m];
            let mut skips = 0;
            for k in 0..i {
                skips += skip(sq, p[k]);
            }
            s += binomial(sq as usize - skips, m - i + 1);
        }
        idx += s * ei.factor[i];
        i += t;
    }

    idx
}

fn decompress_pairs(d: &PairsData, idx: usize) -> i32 {
    if d.idx_bits == 0 {
        return d.const_val as i32;
    }

    let main_idx = idx >> d.idx_bits;
    let mut lit_idx  =
        (idx as isize & ((1isize << d.idx_bits) - 1))
        - (1isize << (d.idx_bits - 1));
    let mut block = u32::from_le(d.index_table[main_idx].block) as usize;
    let idx_offset = u16::from_le(d.index_table[main_idx].offset);
    lit_idx += idx_offset as isize;

    while lit_idx < 0 {
        block -= 1;
        lit_idx += d.size_table[block] as isize + 1;
    }
    while lit_idx > d.size_table[block] as isize {
        lit_idx -= d.size_table[block] as isize + 1;
        block += 1;
    }

    let mut ptr = &d.data[block << d.block_size] as *const u8 as *const u32;

    let mut code = unsafe { u64::from_be(*(ptr as *const u64)) };
    ptr = unsafe { ptr.offset(2) };
    let mut bit_cnt = 0;
    let mut sym;
    loop {
        let mut l = 0;
        while code < d.base[l] {
            l += 1;
        }
        sym = u16::from_le(d.offset[l]) as usize;
        let l2 = l + d.min_len as usize;
        sym += ((code - d.base[l]) >> (64 - l2)) as usize;
        if lit_idx < d.sym_len[sym] as isize + 1 {
            break;
        }
        lit_idx -= d.sym_len[sym] as isize + 1;
        code <<= l2;
        bit_cnt += l2;
        if bit_cnt >= 32 {
            bit_cnt -= 32;
            code |= (unsafe { u32::from_be(*ptr) } as u64) << bit_cnt;
            ptr = unsafe { ptr.offset(1) };
        }
    }

    while d.sym_len[sym] != 0 {
        let w = &d.sym_pat[sym];
        let s1 = s1(w);
        if lit_idx < d.sym_len[s1] as isize + 1 {
            sym = s1;
        } else {
            lit_idx -= d.sym_len[s1] as isize + 1;
            sym = s2(w);
        }
    }

    s1(&d.sym_pat[sym]) as i32
}
