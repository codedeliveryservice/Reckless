use game::{lookup, Bitboard, Board, Color, Piece, Score, Square};

/// Evaluates the mobility difference between the two players.
///
/// The player's mobility depends on the number of squares that their pieces can move to.
/// It can also be thought of as a square control.
pub fn evaluate(board: &Board) -> Score {
    evaluate_color(board, Color::White) - evaluate_color(board, Color::Black)
}

fn evaluate_color(board: &Board, color: Color) -> Score {
    evaluate_piece(board, color, Piece::Knight, |count| (count - 4) * 4)
        + evaluate_piece(board, color, Piece::Bishop, |count| (count - 6) * 5)
        + evaluate_piece(board, color, Piece::Rook, |count| (count - 7) * 2)
        + evaluate_piece(board, color, Piece::Queen, |count| (count - 10) * 2)
}

fn evaluate_piece<F>(board: &Board, color: Color, piece: Piece, f: F) -> Score
where
    F: Fn(i32) -> i32,
{
    let occupancies = board.them() | board.us();
    let mut score = 0;
    for square in board.of(piece, color) {
        score += f(get_attacks(square, piece, occupancies).count() as i32);
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
