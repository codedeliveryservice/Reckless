use game::{Board, Move, Piece};

/// Represents the Most Valuable Victim – Least Valuable Attacker heuristic table.
///
/// Indexed by `[attacker][victim]`.
///
/// ```md
/// Victim    →  Pawn  Knight  Bishop    Rook   Queen    King
/// Attacker  ↓
/// Pawn          105     205     305     405     505     605
/// Knight        104     204     304     404     504     604
/// Bishop        103     203     303     403     503     603
/// Rook          102     202     302     402     502     602
/// Queen         101     201     301     401     501     601
/// King          100     200     300     400     500     600
/// ```
const MVV_LVA: [[u32; 6]; 6] = [
    [105, 205, 305, 405, 505, 605],
    [104, 204, 304, 404, 504, 604],
    [103, 203, 303, 403, 503, 603],
    [102, 202, 302, 402, 502, 602],
    [101, 201, 301, 401, 501, 601],
    [100, 200, 300, 400, 500, 600],
];

/// Scores capture move based on the MVV LVA (Most Valuable Victim – Least Valuable Attacker) heuristic.
pub fn score_mvv_lva(board: &Board, mv: Move) -> u32 {
    let start = board.get_piece(mv.start()).unwrap();

    // This trick handles en passant captures by unwrapping as a pawn for a default piece,
    // since the target square for en passant is different from the captured piece's square
    let target = board.get_piece(mv.target()).unwrap_or(Piece::Pawn);

    MVV_LVA[start][target]
}
