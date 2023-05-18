use game::{Board, Color, Piece, Score};

const MATERIAL: [i32; Piece::NUM - 1] = [100, 300, 325, 500, 900];

/// Evaluates the material difference between the two players in favor of `Color::White`.
pub fn evaluate(board: &Board) -> Score {
    let mut score = 0;
    for index in 0..5 {
        let piece = Piece::from(index);
        let count = board.of(piece, Color::White).count() - board.of(piece, Color::Black).count();
        score += count as i32 * MATERIAL[piece];
    }
    Score(score)
}
