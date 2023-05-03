use game::{Board, Color, Piece, Score};

const SCORES: [i32; 5] = [100, 300, 325, 500, 900];

/// Evaluates the material difference between the two players in favor of `Color::White`.
pub fn evaluate(board: &Board) -> Score {
    let mut score = 0;
    for index in 0..5 {
        let piece = Piece::from(index);
        let count = board.of(piece, Color::White).count() - board.of(piece, Color::Black).count();
        score += count as i32 * SCORES[piece];
    }
    Score(score)
}
