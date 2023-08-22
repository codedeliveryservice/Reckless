use crate::core::{Castling, Move, Piece, Square};

/// Contains information required to unmake the move and irreversible aspects of a chess
/// position that cannot be restored by unmaking a move from the previous position,
/// such as an en passant target square, castling rights, etc.
#[derive(Default, Clone, Copy)]
pub(super) struct State {
    pub previous_move: Move,
    pub captured_piece: Option<Piece>,
    pub en_passant: Option<Square>,
    pub castling: Castling,
}
