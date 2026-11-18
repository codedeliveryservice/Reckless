//! The tables' own facts over Reckless's types (their piece order, material
//! keys, name letters and pawn squares), the outcome classes, and `View`, a
//! board as the tables read it.

use crate::{
    board::Board,
    types::{Bitboard, CastlingKind, Color, Piece, PieceType, Square},
};

/// The tables' piece order: white's men, then black's, each king first and
/// then by strength.
pub const TABLE_PIECES: [Piece; 12] = [
    Piece::WhiteKing,
    Piece::WhiteQueen,
    Piece::WhiteRook,
    Piece::WhiteBishop,
    Piece::WhiteKnight,
    Piece::WhitePawn,
    Piece::BlackKing,
    Piece::BlackQueen,
    Piece::BlackRook,
    Piece::BlackBishop,
    Piece::BlackKnight,
    Piece::BlackPawn,
];

/// A piece type's place in a side's run of `TABLE_PIECES`: king 0 .. pawn 5.
pub const fn type_order(pt: PieceType) -> usize {
    match pt {
        PieceType::King => 0,
        PieceType::Queen => 1,
        PieceType::Rook => 2,
        PieceType::Bishop => 3,
        PieceType::Knight => 4,
        _ => 5,
    }
}

/// A piece's place in `TABLE_PIECES`.
pub const fn table_order(p: Piece) -> usize {
    p.color() as usize * 6 + type_order(p.piece_type())
}

/// A piece type's letter in table names.
pub const fn piece_type_char(pt: PieceType) -> char {
    match pt {
        PieceType::Pawn => 'P',
        PieceType::Knight => 'N',
        PieceType::Bishop => 'B',
        PieceType::Rook => 'R',
        PieceType::Queen => 'Q',
        _ => 'K',
    }
}

/// The same piece of the other colour.
pub fn flip_color(p: Piece) -> Piece {
    Piece::new(!p.color(), p.piece_type())
}

pub const MAX_MAN: usize = 32;

/// Squares a pawn can occupy: ranks 2..7, numbered 0..47 from a2.
pub const PAWN_SQUARE_NB: usize = 48;

/// The squares a piece of type `pt` can stand on, in the tables' numbering:
/// 64, or for a pawn 48.
pub const fn possible_square_count(pt: PieceType) -> usize {
    if matches!(pt, PieceType::Pawn) { PAWN_SQUARE_NB } else { Square::NUM }
}

/// Possible square `index` of a piece of type `pt`.
pub const fn possible_square(pt: PieceType, index: usize) -> Square {
    Square::new((if matches!(pt, PieceType::Pawn) { 8 + index } else { index }) as u8)
}

/// Each piece's weight in a material key, as table files store it: base 9,
/// pawn to queen, black's five digits above white's.
const fn mat_key_weight(p: Piece) -> u32 {
    let white = match p.piece_type() {
        PieceType::Pawn => 1,
        PieceType::Knight => 9,
        PieceType::Bishop => 81,
        PieceType::Rook => 729,
        PieceType::Queen => 6561,
        _ => 0,
    };
    if matches!(p.color(), Color::White) { white } else { white * 59049 }
}

/// A table's material: the base-9 key, whether it holds an opposing pawn pair,
/// and its castling rights. Ordered by key, then pair, then rights.
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MaterialKey {
    key: u32,
    pair_flag: bool,
    castling_code: u8,
}

impl MaterialKey {
    pub const fn add_piece(&mut self, p: Piece) {
        self.key += mat_key_weight(p);
    }

    pub const fn add_pair(&mut self) {
        self.pair_flag = true;
    }

    /// Each right as its rook, and the rights' code.
    pub fn add_castling(&mut self, white_rights: usize, black_rights: usize) {
        debug_assert!(white_rights <= 2 && black_rights <= 2);
        for _ in 0..white_rights {
            self.add_piece(Piece::WhiteRook);
        }
        for _ in 0..black_rights {
            self.add_piece(Piece::BlackRook);
        }
        self.castling_code = (white_rights * 3 + black_rights) as u8;
    }

    pub const fn value(self) -> u32 {
        self.key
    }
}

/// A board as the tables read it, optionally mirrored: colours swapped and
/// ranks flipped, which is how a position is read from the frame of the other
/// colour. No en passant square: that is the caller's.
#[derive(Copy, Clone)]
pub struct View<'a> {
    board: &'a Board,
    mirrored: bool,
}

impl<'a> View<'a> {
    pub const fn new(board: &'a Board) -> Self {
        Self { board, mirrored: false }
    }

    pub const fn mirror(self) -> Self {
        Self { board: self.board, mirrored: !self.mirrored }
    }

    fn color(self, c: Color) -> Color {
        if self.mirrored { !c } else { c }
    }

    pub fn turn(self) -> Color {
        self.color(self.board.side_to_move())
    }

    pub fn piece_bb(self, p: Piece) -> Bitboard {
        let b = self.board.colored_pieces(self.color(p.color()), p.piece_type());
        if self.mirrored { Bitboard(b.0.swap_bytes()) } else { b }
    }

    pub fn king_square(self, c: Color) -> Square {
        self.piece_bb(Piece::new(c, PieceType::King)).lsb()
    }

    pub fn any_castling(self) -> bool {
        self.board.castling().raw() != 0
    }

    pub fn can_castle(self, c: Color, h_side: bool) -> bool {
        self.board.castling().is_allowed(self.right(c, h_side))
    }

    pub fn castling_rook_square(self, c: Color, h_side: bool) -> Square {
        let s = self.board.castling_rook(self.right(c, h_side));
        if self.mirrored { s.flip_rank() } else { s }
    }

    fn right(self, c: Color, h_side: bool) -> CastlingKind {
        CastlingKind::KINDS[self.color(c)][h_side as usize]
    }
}

/// Semantic 5-class outcome for the side to move.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum WdlEntry {
    Lose = 0,
    BlessedLoss = 1,
    Draw = 2,
    CursedWin = 3,
    Win = 4,
    Illegal = 7,
}

/// On-disk 4-bit code: the five classes share `WdlEntry`'s values, plus two
/// markers for a WIN/LOSE at the 50mr edge. Only the dropped-frame derive reads
/// the markers; everything else turns a stored code into a class via
/// `wdl_from_storage`. The distinct type keeps a marker out of semantic code.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum WdlStored {
    Lose = 0,
    BlessedLoss = 1,
    Draw = 2,
    CursedWin = 3,
    Win = 4,
    BoundaryLoss = 5,
    BoundaryWin = 6,
    Illegal = 7,
}

impl WdlStored {
    pub const fn from_bits(v: u8) -> Self {
        match v {
            0 => Self::Lose,
            1 => Self::BlessedLoss,
            2 => Self::Draw,
            3 => Self::CursedWin,
            4 => Self::Win,
            5 => Self::BoundaryLoss,
            6 => Self::BoundaryWin,
            _ => Self::Illegal,
        }
    }

    /// The stored code with a class's value.
    pub const fn from_entry(w: WdlEntry) -> Self {
        Self::from_bits(w as u8)
    }
}

/// A win or loss the 50-move rule doesn't touch.
pub const fn is_decisive_class(w: WdlEntry) -> bool {
    matches!(w, WdlEntry::Win | WdlEntry::Lose)
}

pub const fn wdl_from_storage(s: WdlStored) -> WdlEntry {
    match s {
        WdlStored::Lose | WdlStored::BoundaryLoss => WdlEntry::Lose,
        WdlStored::BlessedLoss => WdlEntry::BlessedLoss,
        WdlStored::Draw => WdlEntry::Draw,
        WdlStored::CursedWin => WdlEntry::CursedWin,
        WdlStored::Win | WdlStored::BoundaryWin => WdlEntry::Win,
        WdlStored::Illegal => WdlEntry::Illegal,
    }
}

/// No 50-move clock: DTM50's unclocked layer.
pub const IGNORE_50MR: u32 = u32::MAX;
pub const DRAWN_AT_LAYER: u16 = 0xFFFF;
pub const DTZ_MAX_NON_CURSED: u16 = 100;
/// Clocks 0..99 have a DTM50 layer.
pub const DTM50_HMC_COUNT: u32 = 100;

pub const WDL_MAGIC: u32 = 0x9bd1e3a6;
pub const DTZ_MAGIC: u32 = 0x2ec8b161;
pub const DTC_MAGIC: u32 = 0x2ec8b17e;
pub const DTM_MAGIC: u32 = 0xab57c134;
pub const DTM50_MAGIC: u32 = 0xab57c151;

pub const WDL_ENTRY_PACK_RATIO: usize = 2;
pub const WDL_ENTRY_BITS: usize = 4;
