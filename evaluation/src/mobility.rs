use game::{lookup, Bitboard, Board, Color, Piece, Score, Square};

use crate::weights::MOBILITY_SCORES;

/// Evaluates the mobility difference between the two players.
///
/// The player's mobility depends on the number of squares that their pieces can move to.
/// It can also be thought of as a square control.
pub fn evaluate(board: &Board) -> Score {
    evaluate_color(board, Color::White) - evaluate_color(board, Color::Black)
}

fn evaluate_color(board: &Board, color: Color) -> Score {
    evaluate_piece(board, color, Piece::Knight)
        + evaluate_piece(board, color, Piece::Bishop)
        + evaluate_piece(board, color, Piece::Rook)
        + evaluate_piece(board, color, Piece::Queen)
}

fn evaluate_piece(board: &Board, color: Color, piece: Piece) -> Score {
    let occupancies = board.them() | board.us();
    let mut score = 0;
    for square in board.of(piece, color) {
        let count = get_attacks(square, piece, occupancies).count();
        score += MOBILITY_SCORES[piece as usize - 1][count as usize];
    }
    Score(score)
}

fn get_attacks(square: Square, piece: Piece, occupancies: Bitboard) -> Bitboard {
    match piece {
        Piece::Knight => lookup::knight_attacks(square),
        Piece::Bishop => lookup::bishop_attacks(square, occupancies),
        Piece::Rook => lookup::rook_attacks(square, occupancies),
        Piece::Queen => lookup::queen_attacks(square, occupancies),
        _ => panic!("Invalid piece"),
    }
}
