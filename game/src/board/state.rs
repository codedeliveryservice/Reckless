use crate::core::{Castling, Move, Piece, Square, Zobrist};

/// Contains information required to unmake the move and irreversible aspects of a chess
/// position that cannot be restored by unmaking a move from the previous position,
/// such as an en passant target square, castling rights, etc.
#[derive(Default, Clone, Copy)]
pub struct State {
    pub(super) previous_move: Option<Move>,
    pub(super) captured_piece: Option<Piece>,
    pub en_passant: Option<Square>,
    pub castling: Castling,
    pub hash_key: Zobrist,
}
