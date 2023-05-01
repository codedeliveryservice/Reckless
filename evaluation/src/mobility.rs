use game::{lookup, Bitboard, Board, Color, Piece, Score, Square};

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
    let mut score = 0;
    for square in board.of(piece, color) {
        score += f(get_attacks(board, square, piece).count() as i32);
    }
    Score(score)
}

fn get_attacks(board: &Board, square: Square, piece: Piece) -> Bitboard {
    match piece {
        Piece::Knight => lookup::knight_attacks(square),
        Piece::Bishop => lookup::bishop_attacks(square, board.them() | board.us()),
        Piece::Rook => lookup::rook_attacks(square, board.them() | board.us()),
        Piece::Queen => lookup::queen_attacks(square, board.them() | board.us()),
        _ => panic!("Invalid piece"),
    }
}
