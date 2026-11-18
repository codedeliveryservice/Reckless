//! Position indexing: a table's material (`PieceConfig`), its index layout
//! (`PositionIndexConfig`), and a position's index within it. Every
//! enumeration here must match ChessTB's generator: slice ids are on disk.

use std::sync::OnceLock;

use super::common::*;
use crate::types::{Bitboard, Color, Piece, PieceType, Square};

pub const MAX_PIECE_GROUP_SIZE: usize = 7;

const BINOMIAL: [[u32; 8]; 65] = {
    let mut b = [[0u32; 8]; 65];
    let mut k = 0;
    while k <= 64 {
        b[k][0] = 1;
        let mut n = 1;
        while n <= 7 {
            b[k][n] = if n > k { 0 } else { b[k - 1][n - 1] + b[k - 1][n] };
            n += 1;
        }
        k += 1;
    }
    b
};

fn kings_adjacent(a: Square, b: Square) -> bool {
    a.distance_from(b) == 1
}

/// A square's file and rank, 0..7 each.
fn file_rank(s: Square) -> (u8, u8) {
    (s.file() as u8, s.rank() as u8)
}

/// The squares, a1 first.
fn all_squares() -> impl Iterator<Item = Square> {
    (0..Square::NUM as u8).map(Square::new)
}

/// An opposing pawn pair (lowercase 'p'): a white and a black pawn on the same
/// file, white below black, neither able to pass or promote while the pair
/// stands. Ranks the joint placement: white ranks 2..6, black 3..7, C(6,2) x 8
/// files = 120 placements, file-major, then white rank, then black rank.
pub struct PairGroup {
    white: Vec<Square>,
    black: Vec<Square>,
    inverse: Box<[i32; 64 * 64]>,
}

pub const PAIR_PLACEMENT_COUNT: usize = 120;

impl PairGroup {
    pub fn get() -> &'static Self {
        static PAIR: OnceLock<PairGroup> = OnceLock::new();
        PAIR.get_or_init(|| {
            let mut g = Self {
                white: Vec::new(),
                black: Vec::new(),
                inverse: Box::new([-1; 64 * 64]),
            };
            for file in 0..8 {
                for wr in 1..=5 {
                    for br in wr + 1..=6 {
                        let w = Square::from_rank_file(wr, file);
                        let b = Square::from_rank_file(br, file);
                        g.inverse[Self::inv_key(w, b)] = g.white.len() as i32;
                        g.white.push(w);
                        g.black.push(b);
                    }
                }
            }
            debug_assert_eq!(g.white.len(), PAIR_PLACEMENT_COUNT);
            g
        })
    }

    pub fn table_size(&self) -> usize {
        self.white.len()
    }

    pub fn white_square(&self, i: usize) -> Square {
        self.white[i]
    }

    pub fn black_square(&self, i: usize) -> Square {
        self.black[i]
    }

    /// The caller guarantees a legal pair.
    pub fn index_of(&self, white_sq: Square, black_sq: Square) -> usize {
        let idx = self.inverse[Self::inv_key(white_sq, black_sq)];
        debug_assert!(idx >= 0);
        idx as usize
    }

    /// Same file, white below black, both on ranks the enumeration covers.
    pub fn is_opposing_pair(white_sq: Square, black_sq: Square) -> bool {
        let ((wf, wr), (bf, br)) = (file_rank(white_sq), file_rank(black_sq));
        wf == bf && wr >= 1 && br <= 6 && wr < br
    }

    /// The pair pawns are indistinguishable from free ones on a board, so the
    /// slot is positional: the opposing pair minimal by (file, white rank,
    /// black rank), if there is one.
    pub fn canonical_pair(white_sqs: &[Square], black_sqs: &[Square]) -> Option<(Square, Square)> {
        let mut best: Option<(Square, Square)> = None;
        for &w in white_sqs {
            for &b in black_sqs {
                if !Self::is_opposing_pair(w, b) {
                    continue;
                }
                if best.is_none_or(|(bw, bb)| Self::pair_key(w, b) < Self::pair_key(bw, bb)) {
                    best = Some((w, b));
                }
            }
        }
        best
    }

    const fn inv_key(w: Square, b: Square) -> usize {
        w as usize * 64 + b as usize
    }

    /// (file, white rank, black rank): the order pairs compare in.
    fn pair_key(w: Square, b: Square) -> (u8, u8, u8) {
        (w.file() as u8, w.rank() as u8, b.rank() as u8)
    }
}

/// One side's men that castling rights pin: its king's file, and its castling
/// rooks' files ascending (8 for an absent second rook).
#[derive(Copy, Clone)]
struct SidePlacement {
    king: u8,
    rook: [u8; 2],
}

/// One side's placements, enumerated king file first then rook files, all
/// ascending. Empty for a side holding no rights.
struct SideTable {
    placements: Vec<SidePlacement>,
    inverse: Box<[i32; 9 * 9 * 9]>,
}

impl SideTable {
    fn build(num_rights: usize) -> Self {
        let mut t = Self { placements: Vec::new(), inverse: Box::new([-1; 729]) };
        if num_rights == 0 {
            return t;
        }
        for k in 0..8 {
            if num_rights == 1 {
                for r in (0..8).filter(|&r| r != k) {
                    t.add(SidePlacement { king: k, rook: [r, 8] });
                }
            } else {
                for lo in 0..k {
                    for hi in k + 1..8 {
                        t.add(SidePlacement { king: k, rook: [lo, hi] });
                    }
                }
            }
        }
        t
    }

    /// (king, rook, rook) in base 9; 8 is the absent second rook of a one-right
    /// side, a slot build never fills for two.
    const fn key(p: SidePlacement) -> usize {
        (p.king as usize * 9 + p.rook[0] as usize) * 9 + p.rook[1] as usize
    }

    fn add(&mut self, p: SidePlacement) {
        self.inverse[Self::key(p)] = self.placements.len() as i32;
        self.placements.push(p);
    }

    fn index_of(&self, p: SidePlacement) -> i32 {
        self.inverse[Self::key(p)]
    }
}

const COLORS: [Color; 2] = [Color::White, Color::Black];

/// Joint placement of the men castling rights pin: each unmoved king and the
/// rooks it could still castle with. White's placement outermost, black's
/// within it; stored king-slice ids are built from this order.
struct CastlingGroup {
    rights: [usize; 2],
    side: [SideTable; 2],
}

impl CastlingGroup {
    fn new(rights: [usize; 2]) -> Self {
        debug_assert!(rights[0] <= 2 && rights[1] <= 2 && rights[0] + rights[1] > 0);
        Self {
            rights,
            side: [SideTable::build(rights[0]), SideTable::build(rights[1])],
        }
    }

    fn both_kings_pinned(&self) -> bool {
        self.rights[Color::White] > 0 && self.rights[Color::Black] > 0
    }

    fn b_span(&self) -> usize {
        self.side[Color::Black].placements.len().max(1)
    }

    fn table_size(&self) -> usize {
        self.side[Color::White].placements.len().max(1) * self.b_span()
    }

    fn side(&self, idx: usize, c: Color) -> SidePlacement {
        let within = if c == Color::White { idx / self.b_span() } else { idx % self.b_span() };
        self.side[c].placements[within]
    }

    /// The placement's index, if the men are where some placement has them.
    fn index_of(&self, king: [u8; 2], rooks: [&[u8]; 2]) -> Option<usize> {
        let mut within = [0usize; 2];
        for c in COLORS {
            if self.rights[c] == 0 {
                continue;
            }
            if rooks[c].len() != self.rights[c] {
                return None;
            }
            let mut p = SidePlacement { king: king[c], rook: [8, 8] };
            for (i, &f) in rooks[c].iter().enumerate() {
                p.rook[i] = f;
            }
            if p.rook[1] != 8 && p.rook[1] < p.rook[0] {
                p.rook.swap(0, 1);
            }
            within[c] = usize::try_from(self.side[c].index_of(p)).ok()?;
        }
        Some(within[Color::White] * self.b_span() + within[Color::Black])
    }

    /// The square on `c`'s back rank on `file`.
    fn home_square(file: u8, c: Color) -> Square {
        Square::from_rank_file(0, file).relative_to(c)
    }

    fn king_square(&self, idx: usize, c: Color) -> Square {
        Self::home_square(self.side(idx, c).king, c)
    }

    fn rook_square(&self, idx: usize, c: Color, which: usize) -> Square {
        Self::home_square(self.side(idx, c).rook[which], c)
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SymmetryGroup {
    FileMirror,
    Dihedral8,
}

/// Bit 0 mirrors files, bit 1 ranks, bit 2 transposes (applied last).
type Transform = u8;
const DIAG: Transform = 4;

fn apply_transform(sq: Square, t: Transform) -> Square {
    let (mut f, mut r) = file_rank(sq);
    if t & 1 != 0 {
        f = 7 - f;
    }
    if t & 2 != 0 {
        r = 7 - r;
    }
    if t & 4 != 0 {
        (f, r) = (r, f);
    }
    Square::from_rank_file(r, f)
}

fn sq_on_main_diag(sq: Square) -> bool {
    let (f, r) = file_rank(sq);
    f == r
}

/// The squares a symmetry folds the white king to: the a1-d1-d4 triangle, or
/// files a..d.
fn is_anchor(sym: SymmetryGroup, sq: Square) -> bool {
    let (f, r) = file_rank(sq);
    match sym {
        SymmetryGroup::Dihedral8 => r <= f && f < 4,
        SymmetryGroup::FileMirror => f < 4,
    }
}

pub const SLICE_NONE: i32 = -1;

#[derive(Copy, Clone)]
pub struct PairLookup {
    pub slice_id: i32,
    pub transform: Transform,
    pub has_diag_stabilizer: bool,
}

const fn pair_slot(wk: Square, bk: Square) -> usize {
    wk as usize * 64 + bk as usize
}

/// The king slices: each king pair a symmetry folds to its canonical one, or,
/// for castling materials, each placement of the pinned men (and a free king).
pub struct KingSliceManager {
    pub num_slices: usize,
    pub has_castling: bool,
    pair_lookup: Vec<PairLookup>,
    castling_rights: [usize; 2],
    castling_slice_lookup: Vec<i32>,
    castling_group: Option<CastlingGroup>,
    free_king_stride: usize,
}

impl KingSliceManager {
    pub fn for_symmetry(sym: SymmetryGroup) -> &'static Self {
        static FILE_MIRROR: OnceLock<KingSliceManager> = OnceLock::new();
        static DIHEDRAL_8: OnceLock<KingSliceManager> = OnceLock::new();
        match sym {
            SymmetryGroup::Dihedral8 => DIHEDRAL_8.get_or_init(|| Self::symmetric(sym)),
            SymmetryGroup::FileMirror => FILE_MIRROR.get_or_init(|| Self::symmetric(sym)),
        }
    }

    pub fn for_castling(rights: [usize; 2]) -> &'static Self {
        static TABLE: OnceLock<Vec<Option<KingSliceManager>>> = OnceLock::new();
        let table = TABLE.get_or_init(|| (0..9).map(|i| (i > 0).then(|| Self::castling([i / 3, i % 3]))).collect());
        table[rights[0] * 3 + rights[1]].as_ref().unwrap()
    }

    fn symmetric(sym: SymmetryGroup) -> Self {
        let none = PairLookup {
            slice_id: SLICE_NONE,
            transform: 0,
            has_diag_stabilizer: false,
        };
        let mut lookup = vec![none; Square::NUM * Square::NUM];
        let mut num_slices = 0;

        for wk in all_squares().filter(|&wk| is_anchor(sym, wk)) {
            for bk in all_squares() {
                if bk == wk || kings_adjacent(wk, bk) {
                    continue;
                }
                if sym == SymmetryGroup::Dihedral8 && sq_on_main_diag(wk) {
                    let bk_d = apply_transform(bk, DIAG);
                    if bk_d != bk && bk > bk_d {
                        continue;
                    }
                }
                let stab = sym == SymmetryGroup::Dihedral8 && sq_on_main_diag(wk) && sq_on_main_diag(bk);
                lookup[pair_slot(wk, bk)] = PairLookup {
                    slice_id: num_slices as i32,
                    transform: 0,
                    has_diag_stabilizer: stab,
                };
                num_slices += 1;
            }
        }

        let n_trans = if sym == SymmetryGroup::Dihedral8 { 8 } else { 2 };
        for wk in all_squares() {
            for bk in all_squares() {
                if lookup[pair_slot(wk, bk)].slice_id != SLICE_NONE || wk == bk || kings_adjacent(wk, bk) {
                    continue;
                }
                for t in 0..n_trans {
                    let look = lookup[pair_slot(apply_transform(wk, t), apply_transform(bk, t))];
                    if look.slice_id != SLICE_NONE && look.transform == 0 {
                        lookup[pair_slot(wk, bk)] = PairLookup { transform: t, ..look };
                        break;
                    }
                }
            }
        }

        Self {
            num_slices,
            has_castling: false,
            pair_lookup: lookup,
            castling_rights: [0, 0],
            castling_slice_lookup: Vec::new(),
            castling_group: None,
            free_king_stride: 1,
        }
    }

    fn castling(rights: [usize; 2]) -> Self {
        let group = CastlingGroup::new(rights);
        let stride = if group.both_kings_pinned() { 1 } else { Square::NUM };
        let n = group.table_size();
        let mut lookup = vec![SLICE_NONE; n * stride];
        let mut num_slices = 0;

        for ci in 0..n {
            let mut pinned_king = [Square::None; 2];
            let mut pinned = Bitboard::default();
            for c in COLORS {
                if rights[c] == 0 {
                    continue;
                }
                pinned_king[c] = group.king_square(ci, c);
                pinned |= pinned_king[c].to_bb();
                for i in 0..rights[c] {
                    pinned |= group.rook_square(ci, c, i).to_bb();
                }
            }

            if group.both_kings_pinned() {
                lookup[ci * stride] = num_slices;
                num_slices += 1;
                continue;
            }

            let ksq = pinned_king[if rights[Color::White] > 0 { Color::White } else { Color::Black }];
            for fk in all_squares() {
                if pinned.contains(fk) || kings_adjacent(ksq, fk) {
                    continue;
                }
                lookup[ci * stride + fk as usize] = num_slices;
                num_slices += 1;
            }
        }

        Self {
            num_slices: num_slices as usize,
            has_castling: true,
            pair_lookup: Vec::new(),
            castling_rights: rights,
            castling_slice_lookup: lookup,
            castling_group: Some(group),
            free_king_stride: stride,
        }
    }

    pub fn lookup(&self, wk: Square, bk: Square) -> PairLookup {
        debug_assert!(!self.has_castling);
        self.pair_lookup[pair_slot(wk, bk)]
    }

    pub fn castling_slice_of(&self, wk: Square, bk: Square, rooks: [&[Square]; 2]) -> i32 {
        let group = self.castling_group.as_ref().unwrap();
        let king_sq = [wk, bk];
        let mut king_file = [8u8; 2];
        let mut rook_files = [[8u8; 2]; 2];
        for c in COLORS {
            if rooks[c].len() != self.castling_rights[c] {
                return SLICE_NONE;
            }
            if self.castling_rights[c] == 0 {
                continue;
            }
            let home = Square::from_rank_file(0, 0).relative_to(c).rank() as u8;
            let (kf, kr) = file_rank(king_sq[c]);
            if kr != home {
                return SLICE_NONE;
            }
            king_file[c] = kf;
            for (i, &r) in rooks[c].iter().enumerate() {
                let (rf, rr) = file_rank(r);
                if rr != home {
                    return SLICE_NONE;
                }
                rook_files[c][i] = rf;
            }
        }

        let (w, b) = (self.castling_rights[Color::White], self.castling_rights[Color::Black]);
        let Some(ci) = group.index_of(king_file, [&rook_files[0][..w], &rook_files[1][..b]]) else {
            return SLICE_NONE;
        };

        let free_slot = if group.both_kings_pinned() {
            0
        } else {
            king_sq[if w == 0 { Color::White } else { Color::Black }] as usize
        };
        self.castling_slice_lookup[ci * self.free_king_stride + free_slot]
    }
}

/// Up to `MAX_PIECE_GROUP_SIZE` squares, kept ascending.
#[derive(Copy, Clone, Default)]
pub struct Placement {
    squares: [Square; MAX_PIECE_GROUP_SIZE],
    size: u8,
}

impl Placement {
    pub fn add(&mut self, s: Square) {
        let mut i = self.size as usize;
        while i > 0 && self.squares[i - 1] > s {
            self.squares[i] = self.squares[i - 1];
            i -= 1;
        }
        self.squares[i] = s;
        self.size += 1;
    }

    fn add_unsorted(&mut self, s: Square) {
        self.squares[self.size as usize] = s;
        self.size += 1;
    }

    /// Lifts one square out, keeping the rest in order: how a castling rook
    /// leaves its colour's rook class. A square not here is left alone.
    pub fn remove_square(&mut self, s: Square) {
        let n = self.size as usize;
        let Some(i) = self.squares[..n].iter().position(|&x| x == s) else {
            return;
        };
        self.squares.copy_within(i + 1..n, i);
        self.size -= 1;
    }

    pub fn as_slice(&self) -> &[Square] {
        &self.squares[..self.size as usize]
    }

    pub const fn len(self) -> usize {
        self.size as usize
    }

    fn transformed(self, f: impl Fn(Square) -> Square) -> Self {
        let mut dst = self;
        for s in &mut dst.squares[..self.size as usize] {
            *s = f(*s);
        }
        dst.squares[..self.size as usize].sort_unstable_by_key(|&s| s as u8);
        dst
    }
}

/// The men of one piece, indexed as a combination of their squares.
#[derive(Clone)]
pub struct PieceGroup {
    piece: Piece,
    num_pieces: usize,
    num_legal_squares: usize,
    table_size: u32,
    sq_to_pos: [i8; 64],
    pos_to_sq: [Square; 64],
}

impl PieceGroup {
    fn new(piece: Piece, count: usize) -> Self {
        let n = possible_square_count(piece.piece_type());
        let mut g = Self {
            piece,
            num_pieces: count,
            num_legal_squares: n,
            table_size: BINOMIAL[n][count],
            sq_to_pos: [-1; 64],
            pos_to_sq: [Square::None; 64],
        };
        for i in 0..n {
            let s = possible_square(piece.piece_type(), i);
            g.pos_to_sq[i] = s;
            g.sq_to_pos[s] = i as i8;
        }
        g
    }

    pub fn table_size(&self) -> usize {
        self.table_size as usize
    }

    pub fn piece(&self) -> Piece {
        self.piece
    }

    pub fn compound_index(&self, list: Placement) -> usize {
        debug_assert_eq!(list.len(), self.num_pieces);
        let mut rank = 0u32;
        for (i, &s) in list.as_slice().iter().enumerate() {
            let p = self.sq_to_pos[s];
            debug_assert!(p >= 0);
            rank += BINOMIAL[p as usize][i + 1];
        }
        debug_assert!(rank < self.table_size);
        rank as usize
    }

    fn squares(&self, idx: usize) -> Placement {
        let mut pos = [0usize; MAX_PIECE_GROUP_SIZE];
        let mut rank = idx as u32;
        let mut hi = self.num_legal_squares;
        for k in (1..=self.num_pieces).rev() {
            let mut p = hi - 1;
            while BINOMIAL[p][k] > rank {
                p -= 1;
            }
            pos[k - 1] = p;
            rank -= BINOMIAL[p][k];
            hi = p;
        }
        let mut pl = Placement::default();
        for &p in &pos[..self.num_pieces] {
            pl.add_unsorted(self.pos_to_sq[p]);
        }
        pl
    }
}

type Mask = [u64; 2];

const fn prefix(n: usize) -> Mask {
    [if n >= 64 { !0 } else { (1 << n) - 1 }, if n <= 64 { 0 } else { (1 << (n - 64)) - 1 }]
}

/// Emits the surviving cells of the pawn slices in cartesian order (pair,
/// white, black): `emit(cart, bits)` supplies up to 64 cells from `cart`.
/// ChessTB's generator and prober share this.
fn build_pawn_slice_bits(
    pair: Option<&PairGroup>, white: &[Placement], black: &[Placement], white_occ: &[u64], black_occ: &[u64],
    mut emit: impl FnMut(usize, u64),
) {
    if let Some(pair) = pair {
        let all = prefix(pair.table_size());
        let mut white_allowed = [[0u64; 2]; Square::NUM];
        let mut black_allowed = [[0u64; 2]; Square::NUM];
        for p in 0..pair.table_size() {
            let (pw, pb) = (pair.white_square(p), pair.black_square(p));
            for sq in (8..56).map(Square::new) {
                if sq == pw || sq == pb {
                    continue;
                }
                if PairGroup::canonical_pair(&[pw, sq], &[pb]) == Some((pw, pb)) {
                    white_allowed[sq][p >> 6] |= 1 << (p & 63);
                }
                if PairGroup::canonical_pair(&[pw], &[pb, sq]) == Some((pw, pb)) {
                    black_allowed[sq][p >> 6] |= 1 << (p & 63);
                }
            }
        }
        let candidates = |placements: &[Placement], allowed: &[Mask; Square::NUM]| -> Vec<Mask> {
            placements
                .iter()
                .map(|pl| {
                    let mut m = all;
                    for &s in pl.as_slice() {
                        m[0] &= allowed[s][0];
                        m[1] &= allowed[s][1];
                    }
                    m
                })
                .collect()
        };
        let white_candidates = candidates(white, &white_allowed);
        let black_candidates = candidates(black, &black_allowed);
        let mut cart = 0;
        for b in 0..black.len() {
            // Pair indices follow canonical ordering. The designated pair must
            // precede every pair among the additional pawns as well.
            let mut before = [all; Square::NUM];
            for ws in (8..56).map(Square::new) {
                let mut first = pair.table_size();
                for &bs in black[b].as_slice() {
                    if PairGroup::is_opposing_pair(ws, bs) {
                        first = first.min(pair.index_of(ws, bs));
                    }
                }
                before[ws] = prefix(first);
            }
            for w in 0..white.len() {
                if white_occ[w] & black_occ[b] == 0 {
                    let mut survivors = [0u64; 2];
                    for (word, s) in survivors.iter_mut().enumerate() {
                        *s = white_candidates[w][word] & black_candidates[b][word];
                        for &ws in white[w].as_slice() {
                            *s &= before[ws][word];
                        }
                    }
                    emit(cart, survivors[0]);
                    emit(cart + 64, survivors[1]);
                }
                cart += pair.table_size();
            }
        }
    } else {
        // Each black placement excludes every white placement touching any of
        // its squares: a whole word of placements at once.
        let words = white.len().div_ceil(64);
        let mut occupied = vec![vec![0u64; words]; Square::NUM];
        for (w, pl) in white.iter().enumerate() {
            for &s in pl.as_slice() {
                occupied[s as usize][w >> 6] |= 1 << (w & 63);
            }
        }
        let tail = white.len() & 63;
        for (b, pl) in black.iter().enumerate() {
            let mut blocked = vec![0u64; words];
            for &s in pl.as_slice() {
                blocked.iter_mut().zip(&occupied[s as usize]).for_each(|(x, &o)| *x |= o);
            }
            for (word, &blocked) in blocked.iter().enumerate() {
                let mut survivors = !blocked;
                if word + 1 == words && tail != 0 {
                    survivors &= (1 << tail) - 1;
                }
                emit(b * white.len() + word * 64, survivors);
            }
        }
    }
}

#[derive(Copy, Clone, Default)]
struct RankBlock {
    survivors_before: i32,
    bits: u64,
}

/// The pawn slices: the cartesian product of pair, white and black placements
/// less the cells that can't stand, numbered by survivors below.
pub struct PawnSliceManager {
    has_pawns: bool,
    pair_table_size: usize,
    white_table_size: usize,
    has_pair: bool,
    white_group: Option<PieceGroup>,
    black_group: Option<PieceGroup>,
    rank_blocks: Vec<RankBlock>,
}

impl PawnSliceManager {
    fn new(pair: Option<&PairGroup>, white_pawns: Option<&PieceGroup>, black_pawns: Option<&PieceGroup>) -> Self {
        let pair_table_size = pair.map_or(1, PairGroup::table_size);
        let white_table_size = white_pawns.map_or(1, PieceGroup::table_size);
        let black_table_size = black_pawns.map_or(1, PieceGroup::table_size);
        let num_cartesian = pair_table_size * white_table_size * black_table_size;

        let placements_of = |g: Option<&PieceGroup>| -> (Vec<Placement>, Vec<u64>) {
            match g {
                None => (vec![Placement::default()], vec![0]),
                Some(g) => {
                    let pl: Vec<Placement> = (0..g.table_size()).map(|i| g.squares(i)).collect();
                    let occ = pl.iter().map(|p| p.as_slice().iter().fold(0u64, |o, &s| o | s.to_bb().0)).collect();
                    (pl, occ)
                }
            }
        };
        let (white_pl, white_occ) = placements_of(white_pawns);
        let (black_pl, black_occ) = placements_of(black_pawns);

        let mut rank_blocks = vec![RankBlock::default(); num_cartesian.div_ceil(64)];
        build_pawn_slice_bits(pair, &white_pl, &black_pl, &white_occ, &black_occ, |cart, bits| {
            if bits == 0 {
                return;
            }
            let (word, shift) = (cart >> 6, cart & 63);
            rank_blocks[word].bits |= bits << shift;
            if shift != 0 && word + 1 < rank_blocks.len() {
                rank_blocks[word + 1].bits |= bits >> (64 - shift);
            }
        });
        let mut running = 0;
        for block in &mut rank_blocks {
            block.survivors_before = running;
            running += block.bits.count_ones() as i32;
        }

        Self {
            has_pawns: pair.is_some() || white_pawns.is_some() || black_pawns.is_some(),
            pair_table_size,
            white_table_size,
            has_pair: pair.is_some(),
            white_group: white_pawns.cloned(),
            black_group: black_pawns.cloned(),
            rank_blocks,
        }
    }

    pub fn has_pawns(&self) -> bool {
        self.has_pawns
    }

    fn compose(&self, pair: usize, w: usize, b: usize) -> usize {
        if !self.has_pawns {
            return 0;
        }
        let cart = pair + w * self.pair_table_size + b * self.pair_table_size * self.white_table_size;
        let blk = self.rank_blocks[cart >> 6];
        let below = (1u64 << (cart & 63)) - 1;
        debug_assert!(blk.bits & (1 << (cart & 63)) != 0);
        blk.survivors_before as usize + (blk.bits & below).count_ones() as usize
    }

    fn lookup_from_squares(
        &self, pair_sqs: Option<(Square, Square)>, white_sqs: &[Square], black_sqs: &[Square],
    ) -> usize {
        if !self.has_pawns {
            return 0;
        }
        let pair_idx = match (self.has_pair, pair_sqs) {
            (true, Some((w, b))) => PairGroup::get().index_of(w, b),
            _ => 0,
        };
        let index_of = |g: &Option<PieceGroup>, sqs: &[Square]| {
            g.as_ref().map_or(0, |g| {
                let mut pl = Placement::default();
                for &s in sqs {
                    pl.add(s);
                }
                g.compound_index(pl)
            })
        };
        self.compose(pair_idx, index_of(&self.white_group, white_sqs), index_of(&self.black_group, black_sqs))
    }
}

/// The tables' piece classes: one per piece type per colour, black's first,
/// each king, knight, bishop, rook, queen, pawn. A table's non-pawn classes are
/// stored in a permutation of this order, so the numbering is the files'.
pub const PIECE_CLASS_NB: usize = 12;
const BLACK_KINGS: usize = 0;
const WHITE_KINGS: usize = 6;
const BLACK_PAWNS: usize = 5;
const WHITE_PAWNS: usize = 11;

const fn piece_class(p: Piece) -> usize {
    let base = if matches!(p.color(), Color::White) { WHITE_KINGS } else { BLACK_KINGS };
    base + match p.piece_type() {
        PieceType::King => 0,
        PieceType::Knight => 1,
        PieceType::Bishop => 2,
        PieceType::Rook => 3,
        PieceType::Queen => 4,
        _ => 5,
    }
}

/// The strength that decides which colour a table stores as white.
const fn side_strength(pt: PieceType) -> usize {
    match pt {
        PieceType::Queen => 900,
        PieceType::Rook => 500,
        PieceType::Bishop => 330,
        PieceType::Knight => 320,
        PieceType::Pawn => 100,
        _ => 0,
    }
}

/// A table's material, colours ordered as the table stores it. Uppercase
/// letters in its name are ordinary men; lowercase 'p' is an opposing pawn
/// pair (one white and one black pawn locked head-to-head, indexed jointly, at
/// most one per material); lowercase 'r' is a rook that still holds a castling
/// right, with the king it castles with (two per side at most). Castling rooks
/// are not in `pieces`.
#[derive(Clone)]
pub struct PieceConfig {
    pieces: [Piece; MAX_MAN],
    num_pieces: usize,
    has_opposing_pair: bool,
    castling_rights: [usize; 2],
    base_mat_key: MaterialKey,
    mirr_mat_key: MaterialKey,
}

impl PieceConfig {
    pub fn new(pcs: &[Piece], castling_rights: [usize; 2]) -> Self {
        debug_assert!(pcs.iter().filter(|&&p| p == Piece::WhiteKing).count() == 1);
        debug_assert!(pcs.iter().filter(|&&p| p == Piece::BlackKing).count() == 1);
        let mut pieces = [Piece::None; MAX_MAN];
        pieces[..pcs.len()].copy_from_slice(pcs);
        let swapped = Self::sort_pieces(&mut pieces[..pcs.len()], castling_rights);
        let rights = if swapped { [castling_rights[1], castling_rights[0]] } else { castling_rights };

        let mut base_mat_key = MaterialKey::default();
        let mut mirr_mat_key = MaterialKey::default();
        for &p in &pieces[..pcs.len()] {
            base_mat_key.add_piece(p);
            mirr_mat_key.add_piece(flip_color(p));
        }
        if rights[0] + rights[1] > 0 {
            base_mat_key.add_castling(rights[0], rights[1]);
            mirr_mat_key.add_castling(rights[1], rights[0]);
        }
        Self {
            pieces,
            num_pieces: pcs.len(),
            has_opposing_pair: false,
            castling_rights: rights,
            base_mat_key,
            mirr_mat_key,
        }
    }

    /// Orders the colours (stronger side white) and the men (in table order);
    /// true if the colours were swapped.
    fn sort_pieces(pieces: &mut [Piece], castling_rights: [usize; 2]) -> bool {
        let mut score = [0usize; 2];
        for &p in pieces.iter() {
            score[p.color()] += side_strength(p.piece_type());
        }
        for c in COLORS {
            score[c] += castling_rights[c] * side_strength(PieceType::Rook);
        }

        let mut do_swap = score[Color::Black] > score[Color::White];
        if score[Color::Black] == score[Color::White] {
            // Counts by type, compared king first (the tables' order).
            let mut keys = [[0i8; 6]; 2];
            for &p in pieces.iter() {
                keys[p.color()][type_order(p.piece_type())] += 1;
            }
            for c in COLORS {
                keys[c][type_order(PieceType::Rook)] += castling_rights[c] as i8;
            }
            do_swap = if keys[0] != keys[1] {
                keys[1] > keys[0]
            } else {
                // Same men on both sides, so the only thing left to order by is
                // which side's rook kept its right. Fewer white rights is the
                // canonical orientation, matching the key's castle code -- KRKr,
                // not KrKR.
                castling_rights[0] > castling_rights[1]
            };
        }

        if do_swap {
            for p in pieces.iter_mut() {
                *p = flip_color(*p);
            }
        }
        pieces.sort_unstable_by_key(|&p| table_order(p));
        do_swap
    }

    pub fn pieces(&self) -> &[Piece] {
        &self.pieces[..self.num_pieces]
    }

    /// The table's file name, without extension.
    pub fn name(&self) -> String {
        let mut s = String::new();
        let mut seen_white_king = false;
        // Castling rooks go in where they would have sorted: before the first
        // man of their colour that outranks a rook.
        let mut written = [0usize; 2];
        let mut write_castling_rooks_before = |s: &mut String, order: usize| {
            for c in COLORS {
                if written[c] == self.castling_rights[c] || table_order(Piece::new(c, PieceType::Rook)) >= order {
                    continue;
                }
                s.extend(std::iter::repeat_n('r', self.castling_rights[c] - written[c]));
                written[c] = self.castling_rights[c];
            }
        };
        for &p in self.pieces() {
            write_castling_rooks_before(&mut s, table_order(p));
            if p.piece_type() == PieceType::King {
                if seen_white_king && self.has_opposing_pair {
                    s.push('p');
                }
                seen_white_king = true;
            }
            s.push(piece_type_char(p.piece_type()));
        }
        write_castling_rooks_before(&mut s, usize::MAX);
        if self.has_opposing_pair {
            s.push('p');
        }
        s
    }

    fn piece_counts(&self) -> [usize; Piece::NUM] {
        let mut counts = [0; Piece::NUM];
        for &p in self.pieces() {
            counts[p] += 1;
        }
        counts
    }

    pub fn has_opposing_pair(&self) -> bool {
        self.has_opposing_pair
    }

    pub fn has_castling(&self) -> bool {
        self.castling_rights[0] + self.castling_rights[1] > 0
    }

    pub fn castling_rights(&self, c: Color) -> usize {
        self.castling_rights[c]
    }

    pub fn base_material_key(&self) -> MaterialKey {
        self.base_mat_key
    }

    pub fn material_keys(&self) -> (MaterialKey, MaterialKey) {
        (self.base_mat_key, self.mirr_mat_key)
    }

    pub fn min_material_key(&self) -> MaterialKey {
        self.base_mat_key.min(self.mirr_mat_key)
    }

    pub fn mark_opposing_pair(&mut self) {
        self.has_opposing_pair = true;
        self.base_mat_key.add_pair();
        self.mirr_mat_key.add_pair();
    }
}

/// Factorials 0!..8!: a permutation index over n classes is below n!.
const FACTORIAL: [u32; 9] = [1, 1, 2, 6, 24, 120, 720, 5040, 40320];

/// The order a table stores its non-pawn classes in, first varying fastest.
#[derive(Copy, Clone, Default)]
pub struct IndexStorageLayout {
    n: usize,
    order: [usize; PIECE_CLASS_NB],
    radix: [usize; PIECE_CLASS_NB],
}

/// A table's index layout: pawn slice, king slice, then the other classes.
pub struct PositionIndexConfig {
    pub cfg: PieceConfig,
    king_slice_manager: &'static KingSliceManager,
    pawn_slice_manager: PawnSliceManager,
    pair_group: Option<&'static PairGroup>,
    pawn_slice_stride: usize,
    within_slice_size: usize,
    populated_classes: Vec<usize>,
    groups: [Option<PieceGroup>; PIECE_CLASS_NB],
    weights: [usize; PIECE_CLASS_NB],
}

impl PositionIndexConfig {
    /// Tables exist only for materials that fit, so the size checks are asserts.
    pub fn new(ps: &PieceConfig) -> Self {
        let counts = ps.piece_counts();
        let has_pawns = counts[Piece::WhitePawn] > 0 || counts[Piece::BlackPawn] > 0 || ps.has_opposing_pair();
        let king_slice_manager = if ps.has_castling() {
            KingSliceManager::for_castling(ps.castling_rights)
        } else if has_pawns {
            KingSliceManager::for_symmetry(SymmetryGroup::FileMirror)
        } else {
            KingSliceManager::for_symmetry(SymmetryGroup::Dihedral8)
        };

        let mut groups: [Option<PieceGroup>; PIECE_CLASS_NB] = Default::default();
        for p in TABLE_PIECES.into_iter().filter(|p| p.piece_type() != PieceType::King) {
            if counts[p] > 0 {
                groups[piece_class(p)] = Some(PieceGroup::new(p, counts[p]));
            }
        }

        let pair_group = ps.has_opposing_pair().then(PairGroup::get);
        let pawn_slice_manager =
            PawnSliceManager::new(pair_group, groups[WHITE_PAWNS].as_ref(), groups[BLACK_PAWNS].as_ref());

        let mut populated_classes = Vec::new();
        let mut weights = [0; PIECE_CLASS_NB];
        let mut w = 1;
        for (i, g) in groups.iter().enumerate() {
            let Some(g) = g else { continue };
            if i == WHITE_PAWNS || i == BLACK_PAWNS {
                continue;
            }
            populated_classes.push(i);
            weights[i] = w;
            w *= g.table_size();
        }

        Self {
            cfg: ps.clone(),
            king_slice_manager,
            pawn_slice_stride: king_slice_manager.num_slices * w,
            within_slice_size: w,
            pawn_slice_manager,
            pair_group,
            populated_classes,
            groups,
            weights,
        }
    }

    fn group(&self, c: usize) -> &PieceGroup {
        self.groups[c].as_ref().unwrap()
    }

    fn is_populated(&self, c: usize) -> bool {
        self.groups[c].is_some()
    }

    pub fn permutation_is_valid(&self, perm: u32) -> bool {
        let n = self.populated_classes.len();
        n <= 8 && perm < FACTORIAL[n]
    }

    /// The storage order of permutation `perm`, which must be valid.
    pub fn storage_layout(&self, perm: u32) -> IndexStorageLayout {
        let n = self.populated_classes.len();
        let mut available = self.populated_classes.clone();
        let mut layout = IndexStorageLayout { n, ..Default::default() };
        let mut idx = perm;
        for i in 0..n {
            let f = FACTORIAL[n - 1 - i];
            let pick = (idx / f) as usize;
            idx %= f;
            layout.order[i] = available.remove(pick);
            layout.radix[i] = self.group(layout.order[i]).table_size();
        }
        layout
    }

    fn placements_from_position(&self, pos: View) -> [Placement; PIECE_CLASS_NB] {
        let mut out = [Placement::default(); PIECE_CLASS_NB];
        out[WHITE_KINGS].add(pos.king_square(Color::White));
        out[BLACK_KINGS].add(pos.king_square(Color::Black));

        for &c in &self.populated_classes {
            for s in pos.piece_bb(self.group(c).piece()) {
                out[c].add(s);
            }
        }
        for (c, p) in [(WHITE_PAWNS, Piece::WhitePawn), (BLACK_PAWNS, Piece::BlackPawn)] {
            if !self.is_populated(c) && self.pair_group.is_none() {
                continue;
            }
            for s in pos.piece_bb(p) {
                out[c].add(s);
            }
        }
        out
    }

    fn transform_placements(&self, placements: &mut [Placement; PIECE_CLASS_NB], t: Transform) {
        if t == 0 {
            return;
        }
        let xform = |s| apply_transform(s, t);
        for c in [WHITE_KINGS, BLACK_KINGS, WHITE_PAWNS, BLACK_PAWNS] {
            if placements[c].len() > 0 {
                placements[c] = placements[c].transformed(xform);
            }
        }
        for &c in &self.populated_classes {
            placements[c] = placements[c].transformed(xform);
        }
    }

    fn canonicalize_placements(&self, placements: &mut [Placement; PIECE_CLASS_NB]) {
        let lookup = self
            .king_slice_manager
            .lookup(placements[WHITE_KINGS].as_slice()[0], placements[BLACK_KINGS].as_slice()[0]);
        if lookup.slice_id == SLICE_NONE {
            return;
        }
        self.transform_placements(placements, lookup.transform);

        if lookup.has_diag_stabilizer {
            let diag = |s| apply_transform(s, DIAG);
            let (mut within_cur, mut within_alt) = (0, 0);
            for &c in &self.populated_classes {
                within_cur += self.weights[c] * self.group(c).compound_index(placements[c]);
                within_alt += self.weights[c] * self.group(c).compound_index(placements[c].transformed(diag));
            }
            if within_alt < within_cur {
                for &c in &self.populated_classes {
                    placements[c] = placements[c].transformed(diag);
                }
            }
        }
    }

    /// The position's index in a colour stored with `layout`; `None` if the
    /// table has no such position.
    pub fn board_index(&self, layout: &IndexStorageLayout, pos: View) -> Option<usize> {
        let mut placements = self.placements_from_position(pos);
        let has_castling = self.cfg.has_castling();
        if !has_castling {
            self.canonicalize_placements(&mut placements);
        }

        let wk = placements[WHITE_KINGS].as_slice()[0];
        let bk = placements[BLACK_KINGS].as_slice()[0];

        let mut rooks = [[Square::None; 2]; 2];
        let mut num_rooks = [0usize; 2];
        if has_castling {
            for c in COLORS {
                // A side holds at most its two rights.
                for h_side in [false, true] {
                    if pos.can_castle(c, h_side) {
                        rooks[c][num_rooks[c]] = pos.castling_rook_square(c, h_side);
                        num_rooks[c] += 1;
                    }
                }
                if num_rooks[c] == 2 && rooks[c][1] < rooks[c][0] {
                    rooks[c].swap(0, 1);
                }
                if num_rooks[c] != self.cfg.castling_rights(c) {
                    return None;
                }
                // The rook class indexes the free rooks only; without them it
                // is empty and so stays.
                for &r in &rooks[c][..num_rooks[c]] {
                    placements[piece_class(Piece::new(c, PieceType::Rook))].remove_square(r);
                }
            }
        }

        let king_slice_id = if has_castling {
            self.king_slice_manager.castling_slice_of(wk, bk, [&rooks[0][..num_rooks[0]], &rooks[1][..num_rooks[1]]])
        } else {
            self.king_slice_manager.lookup(wk, bk).slice_id
        };
        if king_slice_id == SLICE_NONE {
            return None;
        }

        let mut pawn_slice_id = 0;
        if self.pawn_slice_manager.has_pawns() {
            let w_pl = placements[WHITE_PAWNS].as_slice();
            let b_pl = placements[BLACK_PAWNS].as_slice();
            let pair = if self.pair_group.is_some() { PairGroup::canonical_pair(w_pl, b_pl) } else { None };
            let mut free_w = Placement::default();
            let mut free_b = Placement::default();
            for &s in w_pl.iter().filter(|&&s| pair.is_none_or(|(pw, _)| s != pw)) {
                free_w.add_unsorted(s);
            }
            for &s in b_pl.iter().filter(|&&s| pair.is_none_or(|(_, pb)| s != pb)) {
                free_b.add_unsorted(s);
            }
            pawn_slice_id = self.pawn_slice_manager.lookup_from_squares(pair, free_w.as_slice(), free_b.as_slice());
        }

        // Within the slice, the classes in storage order, the first varying fastest.
        let mut within = 0;
        let mut w = 1;
        for i in 0..layout.n {
            let c = layout.order[i];
            within += w * self.group(c).compound_index(placements[c]);
            w *= layout.radix[i];
        }
        Some(pawn_slice_id * self.pawn_slice_stride + king_slice_id as usize * self.within_slice_size + within)
    }
}
