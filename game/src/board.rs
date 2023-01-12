use crate::core::{Bitboard, Color, Move, MoveKind, MoveList, Piece, Square};

use self::{fen::ParseFenError, state::State};

pub mod state;

mod fen;
mod generator;

/// Data structure representing the board and the location of its pieces.
pub struct Board {
    pub turn: Color,
    pieces: [Bitboard; Piece::NUM],
    colors: [Bitboard; Color::NUM],
    history: [State; Self::MAX_SEARCH_DEPTH],
    depth: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct IllegalMoveError;

impl Board {
    const MAX_SEARCH_DEPTH: usize = 64;

    /// Returns the board corresponding to the specified Forsyth–Edwards notation.
    ///
    /// # Errors
    ///
    /// This function will return an error if the given notation is invalid.
    pub fn from_fen(fen: &str) -> Result<Self, ParseFenError> {
        fen::Fen::parse(fen)
    }

    /// Generates all possible pseudo legal moves for the current state of `self`.
    pub fn generate_moves(&self) -> MoveList {
        generator::Generator::generate_moves(self)
    }

    /// Updates the board representation by making the specified `Move`.
    ///
    /// See [Chess Programming Wiki article](https://www.chessprogramming.org/Make_Move) for more information.
    ///
    /// # Panics
    /// Panics if the `Move` contains incorrect information for the current `Board`.
    ///
    /// # Errors
    /// This function will return an error if the `Move` is not allowed by the rules of chess.
    pub fn make_move(&mut self, mv: Move) -> Result<(), IllegalMoveError> {
        self.history[self.depth + 1] = *self.state();
        self.depth += 1;

        self.state_mut().previous_move = Some(mv);

        let start = mv.start();
        let target = mv.target();

        if mv.kind() == MoveKind::EnPassant {
            self.remove_piece(
                Piece::Pawn,
                self.turn.opposite(),
                target.shift(-self.turn.offset()),
            );
        } else if mv.is_capture() {
            let capture = self.get_piece(target).unwrap();
            self.remove_piece(capture, self.turn.opposite(), target);

            self.state_mut().captured_piece = Some(capture);
        }

        if mv.is_promotion() {
            let piece = self.get_piece(start).unwrap();
            self.remove_piece(piece, self.turn, start);
            self.add_piece(mv.get_promotion_piece(), self.turn, target);
        } else {
            let piece = self.get_piece(start).unwrap();
            self.move_piece(piece, self.turn, start, target);
        }

        self.state_mut().en_passant = match mv.kind() == MoveKind::DoublePush {
            true => Some(Square((start.0 + target.0) / 2)),
            false => None,
        };

        if mv.kind() == MoveKind::KingCastling {
            match self.turn {
                Color::White => self.move_piece(Piece::Rook, Color::White, Square::H1, Square::F1),
                Color::Black => self.move_piece(Piece::Rook, Color::Black, Square::H8, Square::F8),
            }
        } else if mv.kind() == MoveKind::QueenCastling {
            match self.turn {
                Color::White => self.move_piece(Piece::Rook, Color::White, Square::A1, Square::D1),
                Color::Black => self.move_piece(Piece::Rook, Color::Black, Square::A8, Square::D8),
            }
        }

        // The move is considered illegal if it exposes the king to an attack after it has been made
        if self.is_in_check() {
            self.turn.reverse();
            self.take_back();

            return Err(IllegalMoveError);
        }

        self.state_mut().castling.update_for_square(start);
        self.state_mut().castling.update_for_square(target);
        self.turn.reverse();

        Ok(())
    }

    /// Restores the board to the previous state after the last move made.
    ///
    /// # Panics
    ///
    /// Panics if there is no previous `Move` or the `Move` is not allowed for the current `Board`.
    pub fn take_back(&mut self) {
        let mv = self.state().previous_move.unwrap();

        self.turn.reverse();

        let start = mv.start();
        let target = mv.target();

        if mv.is_promotion() {
            self.remove_piece(mv.get_promotion_piece(), self.turn, target);
            self.add_piece(Piece::Pawn, self.turn, start);
        } else {
            let piece = self.get_piece(target).unwrap();
            self.move_piece(piece, self.turn, target, start);
        }

        if mv.kind() == MoveKind::EnPassant {
            self.add_piece(
                Piece::Pawn,
                self.turn.opposite(),
                target.shift(-self.turn.offset()),
            );
        } else if mv.is_capture() {
            self.add_piece(
                self.state().captured_piece.unwrap(),
                self.turn.opposite(),
                target,
            );
        }

        if mv.kind() == MoveKind::KingCastling {
            match self.turn {
                Color::White => self.move_piece(Piece::Rook, Color::White, Square::F1, Square::H1),
                Color::Black => self.move_piece(Piece::Rook, Color::Black, Square::F8, Square::H8),
            }
        } else if mv.kind() == MoveKind::QueenCastling {
            match self.turn {
                Color::White => self.move_piece(Piece::Rook, Color::White, Square::D1, Square::A1),
                Color::Black => self.move_piece(Piece::Rook, Color::Black, Square::D8, Square::A8),
            }
        }

        self.depth -= 1;
    }

    /// Returns a reference to the current state of this `Board`.
    #[inline(always)]
    pub fn state(&self) -> &State {
        &self.history[self.depth]
    }

    /// Returns a mutable reference to the current state of this `Board`.
    #[inline(always)]
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.history[self.depth]
    }

    /// Returns a `Bitboard` with friendly pieces for the current state.
    #[inline(always)]
    pub fn us(&self) -> Bitboard {
        self.colors[self.turn as usize]
    }

    /// Returns a `Bitboard` with enemy pieces for the current state.
    #[inline(always)]
    pub fn them(&self) -> Bitboard {
        self.colors[self.turn.opposite() as usize]
    }

    /// Returns a `Bitboard` with friendly pieces of the specified `Piece` type.
    #[inline(always)]
    pub fn our(&self, piece: Piece) -> Bitboard {
        self.pieces[piece as usize] & self.us()
    }

    /// Returns a `Bitboard` with enemy pieces of the specified `Piece` type.
    #[inline(always)]
    pub fn their(&self, piece: Piece) -> Bitboard {
        self.pieces[piece as usize] & self.them()
    }

    /// Finds a piece on the specified `Square` and returns `Some(Piece)`, if found; otherwise `None`.
    #[inline(always)]
    pub fn get_piece(&self, square: Square) -> Option<Piece> {
        for index in 0..Piece::NUM {
            if self.pieces[index].contains(square) {
                return Some(Piece::from(index as u8));
            }
        }

        None
    }

    /// Places a piece of the specified type and color on the square.
    #[inline(always)]
    pub fn add_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.pieces[piece as usize].set(square);
        self.colors[color as usize].set(square);
    }

    /// Removes a piece of the specified type and color from the square.
    #[inline(always)]
    pub fn remove_piece(&mut self, piece: Piece, color: Color, square: Square) {
        self.pieces[piece as usize].clear(square);
        self.colors[color as usize].clear(square);
    }

    /// Moves a piece of the specified type and color from the starting square to the target square.
    #[inline(always)]
    pub fn move_piece(&mut self, piece: Piece, color: Color, start: Square, target: Square) {
        self.add_piece(piece, color, target);
        self.remove_piece(piece, color, start);
    }

    /// Returns `true` if the king of the current turn color is in check.
    pub fn is_in_check(&self) -> bool {
        let square = match self.our(Piece::King).pop() {
            Some(king) => king,
            None => return false,
        };

        self.is_square_attacked(square, self.turn.opposite())
    }

    /// Returns `true` if any piece of the attacker `Color` can attack the `Square`.
    pub fn is_square_attacked(&self, square: Square, color: Color) -> bool {
        use crate::lookup;

        let occupancies = self.them() | self.us();

        let bishop_queen = self.pieces[Piece::Bishop] | self.pieces[Piece::Queen];
        let rook_queen = self.pieces[Piece::Rook] | self.pieces[Piece::Queen];

        let possible_attackers = (lookup::king_attacks(square) & self.pieces[Piece::King])
            | (lookup::knight_attacks(square) & self.pieces[Piece::Knight])
            | (lookup::bishop_attacks(square, occupancies) & bishop_queen)
            | (lookup::rook_attacks(square, occupancies) & rook_queen)
            | (lookup::pawn_attacks(square, color.opposite()) & self.pieces[Piece::Pawn]);

        (possible_attackers & self.colors[color]).is_not_empty()
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            turn: Color::White,
            depth: Default::default(),
            pieces: Default::default(),
            colors: Default::default(),
            history: [Default::default(); Self::MAX_SEARCH_DEPTH],
        }
    }
}
