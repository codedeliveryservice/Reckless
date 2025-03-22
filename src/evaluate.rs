use crate::{board::Board, thread::ThreadData, types::PieceType};

const MATERIAL_VALUES: [i32; 6] = [256, 384, 416, 640, 1280, 0];

/// Calculates the score of the current position from the perspective of the side to move.
pub fn evaluate(td: &mut ThreadData) -> i32 {
    let mut eval = td.nnue.evaluate(&td.board);

    #[cfg(not(feature = "datagen"))]
    {
        let material = material(&td.board);

        eval = (eval * (77000 + material) + td.optimism[td.board.side_to_move()] * (7500 + material)) / 89000;
    }

    eval.clamp(-16384, 16384)
}

fn material(board: &Board) -> i32 {
    [PieceType::Pawn, PieceType::Knight, PieceType::Bishop, PieceType::Rook, PieceType::Queen]
        .iter()
        .map(|&pt| board.pieces(pt).len() as i32 * MATERIAL_VALUES[pt])
        .sum::<i32>()
}
