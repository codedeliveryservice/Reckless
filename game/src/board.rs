use crate::core::{Bitboard, Color, Move, MoveList, Piece, Square};

use self::{change::Change, fen::ParseFenError, state::State};

pub mod state;

mod change;
mod fen;
mod generator;

/// Data structure representing the board and the location of its pieces.
pub struct Board {
    pub turn: Color,
    pub state: State,
    pieces: [Bitboard; Piece::NUM],
    colors: [Bitboard; Color::NUM],
    stack: Vec<Change>,
}

pub struct IllegalMove;

impl Board {
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
    pub fn make_move(&mut self, mv: Move) -> Result<(), IllegalMove> {
        let mut change = Change::new(mv, self.state.clone(), None);

        let start = mv.start();
        let target = mv.target();

        if mv.is_capture() {
            let capture = self.get_piece(target).unwrap();
            self.remove_piece(capture, self.turn.opposite(), target);

            change.capture = Some(capture);
        }

        self.stack.push(change);

        if mv.is_promotion() {
            let piece = self.get_piece(start).unwrap();
            self.remove_piece(piece, self.turn, start);
            self.add_piece(mv.get_promotion_piece(), self.turn, target);
        } else {
            let piece = self.get_piece(start).unwrap();
            self.move_piece(piece, self.turn, start, target);
        }

        // The move is considered illegal if it exposes the king to an attack after it has been made
        if self.is_in_check() {
            self.turn.reverse();
            self.take_back();

            return Err(IllegalMove);
        }

        self.turn.reverse();

        Ok(())
    }

    /// Restores the board to the previous state after the last move made.
    ///
    /// # Panics
    ///
    /// Panics if there is no previous `Move` or the `Move` is not allowed for the current `Board`.
    pub fn take_back(&mut self) {
        let change = self.stack.pop().unwrap();

        self.state = change.state;
        self.turn.reverse();

        let mv = change.mv;

        let start = mv.start();
        let target = mv.target();

        if mv.is_promotion() {
            self.remove_piece(mv.get_promotion_piece(), self.turn, target);
            self.add_piece(Piece::Pawn, self.turn, start);
        } else {
            let piece = self.get_piece(target).unwrap();
            self.move_piece(piece, self.turn, target, start);
        }

        if mv.is_capture() {
            self.add_piece(change.capture.unwrap(), self.turn.opposite(), target);
        }
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

    /// Returns `true` if any piece of the attacker color can attack the `Square`.
    pub fn is_square_attacked(&self, square: Square, attacker: Color) -> bool {
        use crate::lookup::*;

        let attackers = self.colors[attacker];
        let occupancies = self.them() | self.us();

        let bishop_queen = self.pieces[Piece::Bishop] | self.pieces[Piece::Queen];
        let rook_queen = self.pieces[Piece::Rook] | self.pieces[Piece::Queen];

        (king_attacks(square) & self.pieces[Piece::King] & attackers).is_not_empty()
            | (knight_attacks(square) & self.pieces[Piece::Knight] & attackers).is_not_empty()
            | (bishop_attacks(square, occupancies) & bishop_queen & attackers).is_not_empty()
            | (rook_attacks(square, occupancies) & rook_queen & attackers).is_not_empty()
            | (pawn_attacks(square, attacker.opposite()) & self.pieces[Piece::Pawn] & attackers)
                .is_not_empty()
    }
}

impl Default for Board {
    fn default() -> Self {
        Self {
            turn: Color::White,
            state: Default::default(),
            pieces: Default::default(),
            colors: Default::default(),
            stack: Default::default(),
        }
    }
}
