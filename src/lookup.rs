use crate::types::{Bitboard, Color, Move, MoveKind, Piece, PieceType, Square, ZOBRIST};

include!(concat!(env!("OUT_DIR"), "/lookup.rs"));

static mut BETWEEN: [[Bitboard; 64]; 64] = [[Bitboard(0); 64]; 64];

static mut CUCKOO: [u64; 0x2000] = [0; 0x2000];
static mut CUCKOO_MOVES: [Move; 0x2000] = [Move::NULL; 0x2000];

pub fn init() {
    unsafe {
        init_between();
        init_cuckoo();
    }
}

unsafe fn init_between() {
    for a in 0..64 {
        for b in 0..64 {
            let a = Square::new(a);
            let b = Square::new(b);

            if rook_attacks(a, Bitboard(0)).contains(b) {
                BETWEEN[a][b] = rook_attacks(a, b.to_bb()) & rook_attacks(b, a.to_bb());
            }

            if bishop_attacks(a, Bitboard(0)).contains(b) {
                BETWEEN[a][b] = bishop_attacks(a, b.to_bb()) & bishop_attacks(b, a.to_bb());
            }
        }
    }
}

unsafe fn init_cuckoo() {
    fn is_reversible_move(piece: Piece, mv: Move) -> bool {
        match piece.piece_type() {
            PieceType::Knight => knight_attacks(mv.from()),
            PieceType::Bishop => bishop_attacks(mv.from(), Bitboard(0)),
            PieceType::Rook => rook_attacks(mv.from(), Bitboard(0)),
            PieceType::Queen => queen_attacks(mv.from(), Bitboard(0)),
            PieceType::King => king_attacks(mv.from()),
            _ => unreachable!(),
        }
        .contains(mv.to())
    }

    for index in 2..12 {
        let piece = Piece::from_index(index);

        for a in 0..64 {
            for b in (a + 1)..64 {
                let mut mv = Move::new(Square::new(a), Square::new(b), MoveKind::Normal);

                if !is_reversible_move(piece, mv) {
                    continue;
                }

                let mut key = ZOBRIST.pieces[piece][mv.from()] ^ ZOBRIST.pieces[piece][mv.to()] ^ ZOBRIST.side;
                let mut i = h1(key);

                loop {
                    std::mem::swap(&mut CUCKOO[i], &mut key);
                    std::mem::swap(&mut CUCKOO_MOVES[i], &mut mv);

                    if mv == Move::NULL {
                        break;
                    }

                    i = if i == h1(key) { h2(key) } else { h1(key) };
                }
            }
        }
    }
}

pub const fn h1(h: u64) -> usize {
    ((h >> 32) & 0x1fff) as usize
}

pub const fn h2(h: u64) -> usize {
    ((h >> 48) & 0x1fff) as usize
}

pub const fn cuckoo(index: usize) -> u64 {
    unsafe { CUCKOO[index] }
}

pub const fn cuckoo_move(index: usize) -> Move {
    unsafe { CUCKOO_MOVES[index] }
}

pub const fn between(a: Square, b: Square) -> Bitboard {
    unsafe { BETWEEN[a as usize][b as usize] }
}

pub fn pawn_attacks(square: Square, color: Color) -> Bitboard {
    unsafe {
        match color {
            Color::White => Bitboard(*WHITE_PAWN_MAP.get_unchecked(square as usize)),
            Color::Black => Bitboard(*BLACK_PAWN_MAP.get_unchecked(square as usize)),
        }
    }
}

pub fn king_attacks(square: Square) -> Bitboard {
    unsafe { Bitboard(*KING_MAP.get_unchecked(square as usize)) }
}

pub fn knight_attacks(square: Square) -> Bitboard {
    unsafe { Bitboard(*KNIGHT_MAP.get_unchecked(square as usize)) }
}

pub fn rook_attacks(square: Square, occupancies: Bitboard) -> Bitboard {
    unsafe {
        let entry = ROOK_MAGICS.get_unchecked(square as usize);
        let index = magic_index(occupancies, entry);

        Bitboard(*ROOK_MAP.get_unchecked(index as usize))
    }
}

pub fn bishop_attacks(square: Square, occupancies: Bitboard) -> Bitboard {
    unsafe {
        let entry = BISHOP_MAGICS.get_unchecked(square as usize);
        let index = magic_index(occupancies, entry);

        Bitboard(*BISHOP_MAP.get_unchecked(index as usize))
    }
}

pub fn queen_attacks(square: Square, occupancies: Bitboard) -> Bitboard {
    rook_attacks(square, occupancies) | bishop_attacks(square, occupancies)
}

const fn magic_index(occupancies: Bitboard, entry: &MagicEntry) -> u32 {
    let mut hash = occupancies.0 & entry.mask;
    hash = hash.wrapping_mul(entry.magic) >> entry.shift;
    hash as u32 + entry.offset
}
