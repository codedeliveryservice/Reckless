use crate::types::Move;

const RFP_MARGIN: i32 = 75;
const RFP_DEPTH: i32 = 8;

const NMP_DEPTH: i32 = 3;
const NMP_REDUCTION: i32 = 3;

const LRM_MOVES_PLAYED: i32 = 4;
const LMR_DEPTH: i32 = 3;
const LRM_REDUCTION_BASE: i32 = 1;
const LRM_DEPTH_DIVISOR: i32 = 8;
const LRM_MOVES_PLAYED_DIVISOR: i32 = 16;

const QLMP_DEPTH: i32 = 3;
const QLMP_QUIETS_PLAYED: i32 = 5;

impl<'a> super::Searcher<'a> {
    /// If the static evaluation of the position is significantly higher than beta
    /// at low depths, it's likely to be good enough to cause a beta cutoff.
    pub(super) fn reverse_futility_pruning(&self, depth: i32, beta: i32, eval: i32) -> Option<i32> {
        if depth < RFP_DEPTH && eval - RFP_MARGIN * depth > beta {
            return Some(eval);
        }
        None
    }

    /// If giving a free move to the opponent leads to a beta cutoff, it's highly likely
    /// to result in a cutoff after a real move is made, so the node can be pruned.
    pub(super) fn null_move_pruning<const PV: bool>(&mut self, depth: i32, beta: i32, eval: i32) -> Option<i32> {
        if depth >= NMP_DEPTH && eval > beta && !self.board.is_last_move_null() {
            self.board.make_null_move();
            let score = -self.alpha_beta::<PV, false>(-beta, -beta + 1, depth - NMP_REDUCTION);
            self.board.undo_move();

            if score >= beta {
                return Some(beta);
            }
        }
        None
    }
}

/// Calculates the Late Move Reduction (LMR) for a given move.
pub(super) fn calculate_reduction(mv: Move, depth: i32, moves_played: i32, in_check: bool) -> i32 {
    if !mv.is_capture() && !mv.is_promotion() && !in_check && moves_played >= LRM_MOVES_PLAYED && depth >= LMR_DEPTH {
        LRM_REDUCTION_BASE + depth / LRM_DEPTH_DIVISOR + moves_played / LRM_MOVES_PLAYED_DIVISOR
    } else {
        0
    }
}

/// Returns `true` if Quiet late Move Pruning suggests breaking out of the move loop.
///
/// If enough quiet moves have been searched at a low depth, it's unlikely that
/// the remaining moves that are ordered later in move list are going to be better.
pub(super) fn quiet_late_move_pruning(mv: Move, depth: i32, quiets_played: i32) -> bool {
    mv.is_quiet() && depth <= QLMP_DEPTH && quiets_played > QLMP_QUIETS_PLAYED + depth * depth
}
