use game::{Board, Color, Piece, Score};

#[rustfmt::skip]
const MATERIAL: [(i32, i32); Piece::NUM - 1] = [(55, 167), (280, 341), (314, 347), (386, 805), (939, 1395)];

/// Evaluates the material difference between the two players in favor of `Color::White`.
pub fn evaluate(board: &Board) -> (Score, Score) {
    let mut mg = 0;
    let mut eg = 0;

    for (index, (mg_value, eg_value)) in (0..5).zip(MATERIAL) {
        let white = board.of(index.into(), Color::White).count() as i32;
        let black = board.of(index.into(), Color::Black).count() as i32;
        let count = white - black;
        mg += count * mg_value;
        eg += count * eg_value;
    }

    (Score(mg), Score(eg))
}
