use self::{parser::ParseFenError, zobrist::ZOBRIST};
use crate::{
    lookup::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks},
    masks::between,
    nnue::Network,
    parameters::PIECE_VALUES,
    types::{Bitboard, Castling, Color, Move, Piece, PieceType, Square},
};

#[cfg(test)]
mod tests;

mod makemove;
mod movegen;
mod parser;
mod see;
mod zobrist;

/// Contains the same information as a FEN string, used to describe a chess position,
/// along with extra fields for internal use. It's designed to be used as a stack entry,
/// suitable for copying when making/undoing moves.
///
/// Implements the `Copy` trait for efficient memory duplication via bitwise copying.
#[derive(Copy, Clone, Default)]
struct InternalState {
    key: u64,
    pawn_key: u64,
    minor_key: u64,
    major_key: u64,
    non_pawn_keys: [u64; Color::NUM],
    en_passant: Square,
    castling: Castling,
    halfmove_clock: u8,
    captured: Option<Piece>,
    threats: Bitboard,
    pinners: Bitboard,
    checkers: Bitboard,
}

/// A wrapper around the `InternalState` with historical tracking.
#[derive(Clone)]
pub struct Board {
    side_to_move: Color,
    pieces: [Bitboard; PieceType::NUM],
    colors: [Bitboard; Color::NUM],
    mailbox: [Piece; Square::NUM],
    state: InternalState,
    state_stack: Vec<InternalState>,
    game_ply: usize,
    nnue: Network,
}

impl Board {
    /// Returns the board corresponding to the specified Forsyth–Edwards notation.
    pub fn new(fen: &str) -> Result<Self, ParseFenError> {
        fen.parse()
    }

    /// Returns the board corresponding to the starting position.
    pub fn starting_position() -> Self {
        Self::new("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    pub const fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    pub const fn game_ply(&self) -> usize {
        self.game_ply
    }

    /// Returns the Zobrist hash key for the current position.
    pub const fn hash(&self) -> u64 {
        self.state.key
    }

    pub const fn pawn_key(&self) -> u64 {
        self.state.pawn_key
    }

    pub const fn minor_key(&self) -> u64 {
        self.state.minor_key
    }

    pub const fn major_key(&self) -> u64 {
        self.state.major_key
    }

    pub const fn non_pawn_key(&self, color: Color) -> u64 {
        self.state.non_pawn_keys[color as usize]
    }

    pub const fn pinners(&self) -> Bitboard {
        self.state.pinners
    }

    pub const fn checkers(&self) -> Bitboard {
        self.state.checkers
    }

    /// Returns a `Bitboard` for the specified `Color`.
    pub fn colors(&self, color: Color) -> Bitboard {
        self.colors[color]
    }

    /// Returns a `Bitboard` for the specified `Piece` type.
    pub fn pieces(&self, piece_type: PieceType) -> Bitboard {
        self.pieces[piece_type]
    }

    /// Returns a `Bitboard` for all pieces on the board.
    pub fn occupancies(&self) -> Bitboard {
        self.colors(Color::White) | self.colors(Color::Black)
    }

    /// Returns a `Bitboard` for the specified `Piece` type and `Color`.
    pub fn of(&self, piece_type: PieceType, color: Color) -> Bitboard {
        self.pieces(piece_type) & self.colors(color)
    }

    /// Returns a `Bitboard` with friendly pieces for the current state.
    pub fn us(&self) -> Bitboard {
        self.colors(self.side_to_move)
    }

    /// Returns a `Bitboard` with enemy pieces for the current state.
    pub fn them(&self) -> Bitboard {
        self.colors(!self.side_to_move)
    }

    /// Returns a `Bitboard` with friendly pieces of the specified `Piece` type.
    pub fn our(&self, piece_type: PieceType) -> Bitboard {
        self.pieces(piece_type) & self.us()
    }

    /// Returns a `Bitboard` with enemy pieces of the specified `Piece` type.
    pub fn their(&self, piece_type: PieceType) -> Bitboard {
        self.pieces(piece_type) & self.them()
    }

    /// Finds a piece on the specified square, if found; otherwise, `Piece::None`.
    pub fn piece_on(&self, square: Square) -> Piece {
        self.mailbox[square]
    }

    /// Returns `true` if the current side to move has non-pawn material.
    ///
    /// This method is used to minimize the risk of zugzwang when considering the Null Move Heuristic.
    pub fn has_non_pawns(&self) -> bool {
        self.our(PieceType::Pawn) | self.our(PieceType::King) != self.us()
    }

    pub fn increment_game_ply(&mut self) {
        self.game_ply += 1;
    }

    /// Places a piece of the specified type and color on the square.
    pub fn add_piece<const NNUE: bool>(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = piece;
        self.colors[piece.piece_color()].set(square);
        self.pieces[piece.piece_type()].set(square);
        self.update_hash(piece, square);

        if NNUE {
            self.nnue.activate(piece, square);
        }
    }

    /// Removes a piece of the specified type and color from the square.
    pub fn remove_piece<const NNUE: bool>(&mut self, piece: Piece, square: Square) {
        self.mailbox[square] = Piece::None;
        self.colors[piece.piece_color()].clear(square);
        self.pieces[piece.piece_type()].clear(square);
        self.update_hash(piece, square);

        if NNUE {
            self.nnue.deactivate(piece, square);
        }
    }

    pub fn update_hash(&mut self, piece: Piece, square: Square) {
        self.state.key ^= ZOBRIST.pieces[piece][square];

        if piece.piece_type() == PieceType::Pawn {
            self.state.pawn_key ^= ZOBRIST.pieces[piece][square];
        } else {
            self.state.non_pawn_keys[piece.piece_color()] ^= ZOBRIST.pieces[piece][square];
        }

        if [PieceType::Knight, PieceType::Bishop, PieceType::King].contains(&piece.piece_type()) {
            self.state.minor_key ^= ZOBRIST.pieces[piece][square];
        }

        if [PieceType::Rook, PieceType::Queen, PieceType::King].contains(&piece.piece_type()) {
            self.state.major_key ^= ZOBRIST.pieces[piece][square];
        }
    }

    /// Calculates the score of the current position from the perspective of the side to move.
    pub fn evaluate(&self) -> i32 {
        let mut eval = self.nnue.evaluate(self.side_to_move);

        #[cfg(not(feature = "datagen"))]
        {
            let material = PIECE_VALUES[PieceType::Pawn] * self.pieces(PieceType::Pawn).len() as i32
                + PIECE_VALUES[PieceType::Knight] * self.pieces(PieceType::Knight).len() as i32
                + PIECE_VALUES[PieceType::Bishop] * self.pieces(PieceType::Bishop).len() as i32
                + PIECE_VALUES[PieceType::Rook] * self.pieces(PieceType::Rook).len() as i32
                + PIECE_VALUES[PieceType::Queen] * self.pieces(PieceType::Queen).len() as i32;

            eval = eval * (22400 + material) / 32768;
        }

        eval.clamp(-16384, 16384)
    }

    /// Returns `true` if the current position is a known draw by the fifty-move rule or repetition.
    pub fn is_draw(&self) -> bool {
        self.draw_by_repetition() || self.draw_by_fifty_move_rule() || self.draw_by_insufficient_material()
    }

    /// Returns `true` if the current position has already been present at least once
    /// in the board's history.
    ///
    /// This method does not count the number of encounters.
    pub fn draw_by_repetition(&self) -> bool {
        self.state_stack
            .iter()
            .rev()
            .skip(1)
            .step_by(2)
            .take(self.state.halfmove_clock as usize + 1)
            .any(|state| state.key == self.state.key)
    }

    /// Returns `true` if the current position is a known draw by insufficient material:
    /// - Two kings only
    /// - Two kings and one minor piece
    pub fn draw_by_insufficient_material(&self) -> bool {
        match self.occupancies().len() {
            2 => true,
            3 => self.pieces(PieceType::Knight) | self.pieces(PieceType::Bishop) != Bitboard(0),
            _ => false,
        }
    }

    /// Returns `true` if the position is a draw by the fifty-move rule.
    pub const fn draw_by_fifty_move_rule(&self) -> bool {
        self.state.halfmove_clock >= 100
    }

    pub fn in_check(&self) -> bool {
        !self.state.checkers.is_empty()
    }

    pub fn is_threatened(&self, square: Square) -> bool {
        self.state.threats.contains(square)
    }

    pub fn attackers_to(&self, square: Square, occupancies: Bitboard) -> Bitboard {
        rook_attacks(square, occupancies) & (self.pieces(PieceType::Rook) | self.pieces(PieceType::Queen))
            | bishop_attacks(square, occupancies) & (self.pieces(PieceType::Bishop) | self.pieces(PieceType::Queen))
            | pawn_attacks(square, Color::White) & self.of(PieceType::Pawn, Color::Black)
            | pawn_attacks(square, Color::Black) & self.of(PieceType::Pawn, Color::White)
            | knight_attacks(square) & self.pieces(PieceType::Knight)
            | king_attacks(square) & self.pieces(PieceType::King)
    }

    pub fn is_legal(&self, mv: Move) -> bool {
        let from = mv.from();
        let to = mv.to();

        let king = self.our(PieceType::King).lsb();

        if mv.is_en_passant() {
            let occupancies = self.occupancies() ^ from.to_bb() ^ to.to_bb() ^ (to ^ 8).to_bb();

            let diagonal = self.their(PieceType::Bishop) | self.their(PieceType::Queen);
            let orthogonal = self.their(PieceType::Rook) | self.their(PieceType::Queen);

            let diagonal = bishop_attacks(king, occupancies) & diagonal;
            let orthogonal = rook_attacks(king, occupancies) & orthogonal;

            return (orthogonal | diagonal).is_empty();
        }

        if self.piece_on(from).piece_type() == PieceType::King {
            let attackers = self.attackers_to(to, self.occupancies() ^ from.to_bb()) & self.them();
            return attackers.is_empty();
        }

        if self.pinners().contains(from) {
            let along_pin = between(king, from).contains(to) || between(king, to).contains(from);
            return self.checkers().is_empty() && along_pin;
        }

        if self.checkers().multiple() {
            return false;
        }

        if self.checkers().is_empty() {
            return true;
        }

        (self.checkers() | between(king, self.checkers().lsb())).contains(to)
    }

    pub fn update_threats(&mut self) {
        let occupancies = self.occupancies();
        let mut threats = Bitboard::default();

        for square in self.their(PieceType::Pawn) {
            threats |= pawn_attacks(square, !self.side_to_move);
        }

        for square in self.their(PieceType::Knight) {
            threats |= knight_attacks(square);
        }

        for square in self.their(PieceType::Bishop) | self.their(PieceType::Queen) {
            threats |= bishop_attacks(square, occupancies);
        }

        for square in self.their(PieceType::Rook) | self.their(PieceType::Queen) {
            threats |= rook_attacks(square, occupancies);
        }

        self.state.threats = threats | king_attacks(self.their(PieceType::King).lsb());
    }

    pub fn update_king_threats(&mut self) {
        let king = self.our(PieceType::King).lsb();

        self.state.pinners = Bitboard::default();
        self.state.checkers = Bitboard::default();

        self.state.checkers |= pawn_attacks(king, self.side_to_move) & self.their(PieceType::Pawn);
        self.state.checkers |= knight_attacks(king) & self.their(PieceType::Knight);

        let diagonal = self.their(PieceType::Bishop) | self.their(PieceType::Queen);
        let orthogonal = self.their(PieceType::Rook) | self.their(PieceType::Queen);

        let diagonal = bishop_attacks(king, self.them()) & diagonal;
        let orthogonal = rook_attacks(king, self.them()) & orthogonal;

        for square in diagonal | orthogonal {
            let blockers = between(king, square) & self.us();
            match blockers.len() {
                0 => self.state.checkers.set(square),
                1 => self.state.pinners |= blockers,
                _ => (),
            }
        }
    }

    pub fn update_hash_keys(&mut self) {
        self.state.key = 0;
        self.state.pawn_key = 0;
        self.state.minor_key = 0;
        self.state.major_key = 0;
        self.state.non_pawn_keys = [0; Color::NUM];

        for piece in 0..Piece::NUM {
            let piece = Piece::from_index(piece);

            for square in self.of(piece.piece_type(), piece.piece_color()) {
                self.state.key ^= ZOBRIST.pieces[piece][square];

                if piece.piece_type() == PieceType::Pawn {
                    self.state.pawn_key ^= ZOBRIST.pieces[piece][square];
                } else {
                    self.state.non_pawn_keys[piece.piece_color()] ^= ZOBRIST.pieces[piece][square];
                }

                if [PieceType::Knight, PieceType::Bishop, PieceType::King].contains(&piece.piece_type()) {
                    self.state.minor_key ^= ZOBRIST.pieces[piece][square];
                }

                if [PieceType::Rook, PieceType::Queen, PieceType::King].contains(&piece.piece_type()) {
                    self.state.major_key ^= ZOBRIST.pieces[piece][square];
                }
            }
        }

        if self.state.en_passant != Square::None {
            self.state.key ^= ZOBRIST.en_passant[self.state.en_passant];
        }

        if self.side_to_move == Color::White {
            self.state.key ^= ZOBRIST.side;
        }

        self.state.key ^= ZOBRIST.castling[self.state.castling];
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            side_to_move: Color::White,
            state: InternalState::default(),
            pieces: [Bitboard::default(); PieceType::NUM],
            colors: [Bitboard::default(); Color::NUM],
            mailbox: [Piece::None; Square::NUM],
            state_stack: Vec::default(),
            nnue: Network::default(),
            game_ply: 0,
        }
    }
}
