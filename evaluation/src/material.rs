use game::{Board, Color, Piece, Score};

const SCORES: [i32; 5] = [100, 300, 325, 500, 900];

pub fn evaluate_material(board: &Board) -> Score {
    evaluate(board, Color::White) - evaluate(board, Color::Black)
}

fn evaluate(board: &Board, color: Color) -> Score {
    let mut score = 0;
    for index in 0..5 {
        let piece = Piece::from(index);
        score += board.of(piece, color).count() as i32 * SCORES[piece];
    }
    Score(score)
}
