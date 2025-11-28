use crate::types::{Color, Piece, PieceType};

pub fn threat_index(piece: Piece, from: usize, to: usize, target: usize, pov: Color) -> Option<usize> {
    match piece.piece_type() {
        PieceType::Pawn => map_pawn_threat(from, to, target, piece.piece_color() != pov),
        PieceType::Knight => map_knight_threat(from, to, target),
        PieceType::Bishop => map_bishop_threat(from, to, target),
        PieceType::Rook => map_rook_threat(from, to, target),
        PieceType::Queen => map_queen_threat(from, to, target),
        PieceType::King => map_king_threat(from, to, target),
        _ => unreachable!(),
    }
}

fn below(from: usize, to: usize, table: &[u64; 64]) -> usize {
    (table[from] & ((1 << to) - 1)).count_ones() as usize
}

fn target_is(target: usize, piece: PieceType) -> bool {
    target % 6 == piece as usize
}

fn map_pawn_threat(from: usize, to: usize, target: usize, enemy: bool) -> Option<usize> {
    const MAP: [i32; 12] = [0, 1, -1, 2, -1, -1, 3, 4, -1, 5, -1, -1];

    if MAP[target] < 0 || (enemy && to > from && target_is(target, PieceType::Pawn)) {
        return None;
    }

    let id = if to.abs_diff(from) == [9, 7][(to > from) as usize] { 0 } else { 1 };
    let attack = 2 * (from % 8) + id - 1;
    let threat = offsets::PAWN + MAP[target] as usize * indexes::PAWN + (from / 8 - 1) * 14 + attack;
    Some(threat)
}

fn map_knight_threat(from: usize, to: usize, target: usize) -> Option<usize> {
    if to > from && target_is(target, PieceType::Knight) {
        return None;
    }

    let index = indexes::KNIGHT[from] + below(from, to, &attacks::KNIGHT);
    let threat = offsets::KNIGHT + target * indexes::KNIGHT[64] + index;
    Some(threat)
}

fn map_bishop_threat(from: usize, to: usize, target: usize) -> Option<usize> {
    const MAP: [i32; 12] = [0, 1, 2, 3, -1, 4, 5, 6, 7, 8, -1, 9];

    if MAP[target] < 0 || to > from && target_is(target, PieceType::Bishop) {
        return None;
    }

    let index = indexes::BISHOP[from] + below(from, to, &attacks::BISHOP);
    let threat = offsets::BISHOP + MAP[target] as usize * indexes::BISHOP[64] + index;
    Some(threat)
}

fn map_rook_threat(from: usize, to: usize, target: usize) -> Option<usize> {
    const MAP: [i32; 12] = [0, 1, 2, 3, -1, 4, 5, 6, 7, 8, -1, 9];

    if MAP[target] < 0 || to > from && target_is(target, PieceType::Rook) {
        return None;
    }

    let index = indexes::ROOK[from] + below(from, to, &attacks::ROOK);
    let threat = offsets::ROOK + MAP[target] as usize * indexes::ROOK[64] + index;
    Some(threat)
}

fn map_queen_threat(from: usize, to: usize, target: usize) -> Option<usize> {
    if to > from && target_is(target, PieceType::Queen) {
        return None;
    }

    let index = indexes::QUEEN[from] + below(from, to, &attacks::QUEEN);
    let threat = offsets::QUEEN + target * indexes::QUEEN[64] + index;
    Some(threat)
}

fn map_king_threat(from: usize, to: usize, target: usize) -> Option<usize> {
    const MAP: [i32; 12] = [0, 1, 2, 3, -1, -1, 4, 5, 6, 7, -1, -1];

    if MAP[target] < 0 {
        return None;
    }

    let index = indexes::KING[from] + below(from, to, &attacks::KING);
    let threat = offsets::KING + MAP[target] as usize * indexes::KING[64] + index;
    Some(threat)
}

pub mod offsets {
    use super::indexes;

    pub const PAWN: usize = 0;
    pub const KNIGHT: usize = PAWN + 6 * indexes::PAWN;
    pub const BISHOP: usize = KNIGHT + 12 * indexes::KNIGHT[64];
    pub const ROOK: usize = BISHOP + 10 * indexes::BISHOP[64];
    pub const QUEEN: usize = ROOK + 10 * indexes::ROOK[64];
    pub const KING: usize = QUEEN + 12 * indexes::QUEEN[64];
    pub const END: usize = KING + 8 * indexes::KING[64];
}

mod indexes {
    use super::attacks;

    macro_rules! init_add_assign {
        (|$sq:ident, $init:expr, $size:literal | $($rest:tt)+) => {{
            let mut $sq = 0;
            let mut res = [{$($rest)+}; $size + 1];
            let mut val = $init;
            while $sq < $size {
                res[$sq] = val;
                val += {$($rest)+};
                $sq += 1;
            }

            res[$size] = val;

            res
        }};
    }

    pub const PAWN: usize = 84;
    pub const KNIGHT: [usize; 65] = init_add_assign!(|sq, 0, 64| attacks::KNIGHT[sq].count_ones() as usize);
    pub const BISHOP: [usize; 65] = init_add_assign!(|sq, 0, 64| attacks::BISHOP[sq].count_ones() as usize);
    pub const ROOK: [usize; 65] = init_add_assign!(|sq, 0, 64| attacks::ROOK[sq].count_ones() as usize);
    pub const QUEEN: [usize; 65] = init_add_assign!(|sq, 0, 64| attacks::QUEEN[sq].count_ones() as usize);
    pub const KING: [usize; 65] = init_add_assign!(|sq, 0, 64| attacks::KING[sq].count_ones() as usize);
}

mod attacks {
    macro_rules! init {
        (|$sq:ident, $size:literal | $($rest:tt)+) => {{
            let mut $sq = 0;
            let mut res = [{$($rest)+}; $size];
            while $sq < $size {
                res[$sq] = {$($rest)+};
                $sq += 1;
            }
            res
        }};
    }

    const A: u64 = 0x0101_0101_0101_0101;
    const H: u64 = A << 7;

    const DIAGS: [u64; 15] = [
        0x0100_0000_0000_0000,
        0x0201_0000_0000_0000,
        0x0402_0100_0000_0000,
        0x0804_0201_0000_0000,
        0x1008_0402_0100_0000,
        0x2010_0804_0201_0000,
        0x4020_1008_0402_0100,
        0x8040_2010_0804_0201,
        0x0080_4020_1008_0402,
        0x0000_8040_2010_0804,
        0x0000_0080_4020_1008,
        0x0000_0000_8040_2010,
        0x0000_0000_0080_4020,
        0x0000_0000_0000_8040,
        0x0000_0000_0000_0080,
    ];

    pub const KNIGHT: [u64; 64] = init!(|sq, 64| {
        let n = 1 << sq;
        let h1 = ((n >> 1) & 0x7f7f_7f7f_7f7f_7f7f) | ((n << 1) & 0xfefe_fefe_fefe_fefe);
        let h2 = ((n >> 2) & 0x3f3f_3f3f_3f3f_3f3f) | ((n << 2) & 0xfcfc_fcfc_fcfc_fcfc);
        (h1 << 16) | (h1 >> 16) | (h2 << 8) | (h2 >> 8)
    });

    pub const BISHOP: [u64; 64] = init!(|sq, 64| {
        let rank = sq / 8;
        let file = sq % 8;
        DIAGS[file + rank].swap_bytes() ^ DIAGS[7 + file - rank]
    });

    pub const ROOK: [u64; 64] = init!(|sq, 64| {
        let rank = sq / 8;
        let file = sq % 8;
        (0xFF << (rank * 8)) ^ (A << file)
    });

    pub const QUEEN: [u64; 64] = init!(|sq, 64| BISHOP[sq] | ROOK[sq]);

    pub const KING: [u64; 64] = init!(|sq, 64| {
        let mut k = 1 << sq;
        k |= (k << 8) | (k >> 8);
        k |= ((k & !A) >> 1) | ((k & !H) << 1);
        k ^ (1 << sq)
    });
}
