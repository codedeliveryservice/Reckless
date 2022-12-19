use crate::core::{Castling, Square};

/// Contains irreversible aspects of a chess position that cannot be restored by unmaking
/// a move from a previous position, such as an en passant target, castling rights, etc.
///
/// NOTE: At the moment, it's a dummy structure designed to potentially store such information.
#[derive(Default, Clone)]
pub struct State {
    pub en_passant: Option<Square>,
    pub castling: Castling,
}
