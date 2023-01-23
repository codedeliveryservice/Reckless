use game::{
    board::Board,
    core::{Color, Piece},
};

use super::score::Score;

#[rustfmt::skip]
const MATERIAL_SCORES: [MaterialScore; 5] = [
    MaterialScore { piece: Piece::Pawn,   weight: 100 },
    MaterialScore { piece: Piece::Knight, weight: 300 },
    MaterialScore { piece: Piece::Bishop, weight: 325 },
    MaterialScore { piece: Piece::Rook,   weight: 500 },
    MaterialScore { piece: Piece::Queen,  weight: 900 },
];

struct MaterialScore {
    piece: Piece,
    weight: i32,
}

pub fn evaluate_material(board: &Board) -> Score {
    let mut score = Score::EMPTY;

    score += get_score_for_side(board, Color::White);
    score -= get_score_for_side(board, Color::Black);

    score
}

fn get_score_for_side(board: &Board, color: Color) -> Score {
    let mut score = 0;
    for pair in MATERIAL_SCORES {
        score += board.of(pair.piece, color).count() as i32 * pair.weight;
    }
    Score::new(score)
}
