use crate::{
    lookup::attacks,
    types::{Bitboard, Color, Piece, PieceType, Square},
};

#[derive(Copy, Clone)]
struct PiecePair {
    // Bit layout:
    // - bits 8..31: base index contribution for this piece-pair
    // - bits 0..1 : exclusion flags (semi/excluded)
    inner: u32,
}

impl PiecePair {
    fn new(excluded: bool, semi_excluded: bool, base: i32) -> Self {
        Self {
            inner: ((semi_excluded && !excluded) as u32) | ((excluded as u32) << 1) | ((base as u32) << 8),
        }
    }

    fn base(self) -> usize {
        (self.inner >> 8) as usize
    }

    fn excluded(self, attacking: Square, attacked: Square) -> bool {
        let below = ((attacking as u8) < (attacked as u8)) as u8;
        ((self.inner as u8 + below) & 2) != 0
    }
}

static mut PIECE_PAIR_LOOKUP: [[PiecePair; 12]; 12] = [[PiecePair { inner: 0 }; 12]; 12];
static mut PIECE_OFFSET_LOOKUP: [[i32; 64]; 12] = [[0; 64]; 12];
static mut ATTACK_INDEX_LOOKUP: [[[u8; 64]; 64]; 12] = [[[0; 64]; 64]; 12];

pub fn initialize() {
    #[rustfmt::skip]
    const PIECE_INTERACTION_MAP: [[i32; 6]; 6] = [
        [0,  1, -1,  2, -1, -1],
        [0,  1,  2,  3,  4,  5],
        [0,  1,  2,  3, -1,  4],
        [0,  1,  2,  3, -1,  4],
        [0,  1,  2,  3,  4,  5],
        [0,  1,  2,  3, -1, -1],
    ];

    const PIECE_TARGET_COUNT: [i32; 6] = [6, 12, 10, 10, 12, 8];

    let mut offset = 0;
    let mut piece_offset = [0; Piece::NUM];
    let mut offset_table = [0; Piece::NUM];

    for piece_color in [Color::White, Color::Black] {
        for piece_type in 0..PieceType::NUM {
            let piece_type = PieceType::new(piece_type);
            let piece = Piece::new(piece_color, piece_type);

            let mut count = 0;

            for (square, entry) in unsafe { PIECE_OFFSET_LOOKUP[piece].iter_mut().enumerate() } {
                *entry = count;

                if piece_type != PieceType::Pawn || (8..56).contains(&square) {
                    count += attacks(piece, Square::new(square as u8), Bitboard(0)).popcount() as i32;
                }
            }

            piece_offset[piece] = count;
            offset_table[piece] = offset;

            offset += PIECE_TARGET_COUNT[piece_type] * count;
        }
    }

    for attacking in Piece::ALL {
        for attacked in Piece::ALL {
            let attacking_piece = attacking.piece_type();
            let attacking_color = attacking.piece_color();

            let attacked_piece = attacked.piece_type();
            let attacked_color = attacked.piece_color();

            let map = PIECE_INTERACTION_MAP[attacking_piece][attacked_piece];
            let base = offset_table[attacking]
                + ((attacked_color as i32) * (PIECE_TARGET_COUNT[attacking_piece] / 2) + map) * piece_offset[attacking];

            let enemy = attacking_color != attacked_color;
            let semi_excluded = attacking_piece == attacked_piece && (enemy || attacking_piece != PieceType::Pawn);
            let excluded = map < 0;

            unsafe { PIECE_PAIR_LOOKUP[attacking][attacked] = PiecePair::new(excluded, semi_excluded, base) };
        }
    }

    for piece in Piece::ALL {
        for (from, row) in unsafe { ATTACK_INDEX_LOOKUP[piece].iter_mut().enumerate() } {
            let attacks = attacks(piece, Square::new(from as u8), Bitboard(0));

            for (to, entry) in row.iter_mut().enumerate() {
                *entry = (Bitboard((1u64 << to) - 1) & attacks).popcount() as u8;
            }
        }
    }
}

pub fn threat_index(
    piece: Piece, mut from: Square, attacked: Piece, mut to: Square, mirrored: bool, pov: Color,
) -> Option<usize> {
    let flip = (7 * (mirrored as u8)) ^ (56 * (pov as u8));

    from ^= flip;
    to ^= flip;

    let attacking = Piece::new(Color::new((piece.piece_color() as u8) ^ (pov as u8)), piece.piece_type());
    let attacked = Piece::new(Color::new((attacked.piece_color() as u8) ^ (pov as u8)), attacked.piece_type());

    unsafe {
        let pair = PIECE_PAIR_LOOKUP[attacking][attacked];
        if pair.excluded(from, to) {
            return None;
        }

        let index = pair.base()
            + PIECE_OFFSET_LOOKUP[attacking][from] as usize
            + ATTACK_INDEX_LOOKUP[attacking][from][to] as usize;

        Some(index)
    }
}
