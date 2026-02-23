use crate::{
    board::Board,
    lookup::{attacks, bishop_attacks, king_attacks, knight_attacks, pawn_attacks, ray_pass, rook_attacks},
    nnue::ThreatAccumulator,
    nnue::accumulator::ThreatDelta,
    types::{Bitboard, Color, Piece, PieceType, Square},
};

pub fn push_threats_on_change(accum: &mut ThreatAccumulator, board: &Board, piece: Piece, square: Square, add: bool) {
    push_threats_single(accum, board, board.occupancies(), piece, square, add);
}

pub fn push_threats_on_move(accum: &mut ThreatAccumulator, board: &Board, piece: Piece, from: Square, to: Square) {
    let occupancies = board.occupancies() ^ to.to_bb();
    push_threats_single(accum, board, occupancies, piece, from, false);
    push_threats_single(accum, board, occupancies, piece, to, true);
}

fn push_threats_single(
    accum: &mut ThreatAccumulator, board: &Board, occupancies: Bitboard, piece: Piece, square: Square, add: bool,
) {
    let deltas = &mut accum.delta;

    let attacked = attacks(piece, square, occupancies) & occupancies;
    for to in attacked {
        deltas.push(ThreatDelta::new(piece, square, board.piece_on(to), to, add));
    }

    let rook_attacks = rook_attacks(square, occupancies);
    let bishop_attacks = bishop_attacks(square, occupancies);
    let queen_attacks = rook_attacks | bishop_attacks;

    let diagonal = (board.pieces(PieceType::Bishop) | board.pieces(PieceType::Queen)) & bishop_attacks;
    let orthogonal = (board.pieces(PieceType::Rook) | board.pieces(PieceType::Queen)) & rook_attacks;

    for from in (diagonal | orthogonal) & occupancies {
        let sliding_piece = board.piece_on(from);
        let threatened = ray_pass(from, square) & occupancies & queen_attacks;

        if let Some(to) = threatened.into_iter().next() {
            deltas.push(ThreatDelta::new(sliding_piece, from, board.piece_on(to), to, !add));
        }

        deltas.push(ThreatDelta::new(sliding_piece, from, piece, square, add));
    }

    let black_pawns = board.of(PieceType::Pawn, Color::Black) & pawn_attacks(square, Color::White);
    let white_pawns = board.of(PieceType::Pawn, Color::White) & pawn_attacks(square, Color::Black);

    let knights = board.pieces(PieceType::Knight) & knight_attacks(square);
    let kings = board.pieces(PieceType::King) & king_attacks(square);

    for from in (black_pawns | white_pawns | knights | kings) & occupancies {
        deltas.push(ThreatDelta::new(board.piece_on(from), from, piece, square, add));
    }
}

pub fn push_threats_on_mutate(
    accum: &mut ThreatAccumulator, board: &Board, old_piece: Piece, new_piece: Piece, square: Square,
) {
    let deltas = &mut accum.delta;

    let occupancies = board.occupancies();

    let attacked = attacks(old_piece, square, occupancies) & occupancies;
    for to in attacked {
        deltas.push(ThreatDelta::new(old_piece, square, board.piece_on(to), to, false));
    }
    let attacked = attacks(new_piece, square, occupancies) & occupancies;
    for to in attacked {
        deltas.push(ThreatDelta::new(new_piece, square, board.piece_on(to), to, true));
    }

    let rook_attacks = rook_attacks(square, occupancies);
    let bishop_attacks = bishop_attacks(square, occupancies);

    let diagonal = (board.pieces(PieceType::Bishop) | board.pieces(PieceType::Queen)) & bishop_attacks;
    let orthogonal = (board.pieces(PieceType::Rook) | board.pieces(PieceType::Queen)) & rook_attacks;

    let black_pawns = board.of(PieceType::Pawn, Color::Black) & pawn_attacks(square, Color::White);
    let white_pawns = board.of(PieceType::Pawn, Color::White) & pawn_attacks(square, Color::Black);

    let knights = board.pieces(PieceType::Knight) & knight_attacks(square);
    let kings = board.pieces(PieceType::King) & king_attacks(square);

    for from in black_pawns | white_pawns | knights | kings | diagonal | orthogonal {
        deltas.push(ThreatDelta::new(board.piece_on(from), from, old_piece, square, false));
        deltas.push(ThreatDelta::new(board.piece_on(from), from, new_piece, square, true));
    }
}
