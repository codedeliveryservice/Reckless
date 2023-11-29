use crate::types::Move;

const RFP_MARGIN: i32 = 75;
const RFP_DEPTH: i32 = 8;

const NMP_DEPTH: i32 = 3;
const NMP_REDUCTION: i32 = 3;
const NMP_DIVISOR: i32 = 4;

const LMR_MOVES_PLAYED: i32 = 4;
const LMR_DEPTH: i32 = 3;
const LMR_BASE: f64 = 0.75;
const LMR_DIVISOR: f64 = 2.25;
const LMR_HISTORY_DIVISOR: i32 = 200;

const QLMP_DEPTH: i32 = 3;
const QLMP_QUIETS_PLAYED: i32 = 5;

const FUTILITY_DEPTH: i32 = 5;
const FUTILITY_MARGIN: i32 = 125;
const FUTILITY_FIXED_MARGIN: i32 = 50;

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
        if depth >= NMP_DEPTH && eval > beta && !self.board.is_last_move_null() && self.board.has_non_pawn_material() {
            self.board.make_null_move();
            let score = -self.alpha_beta::<PV, false>(-beta, -beta + 1, depth - NMP_REDUCTION - depth / NMP_DIVISOR);
            self.board.undo_move();

            if score >= beta {
                return Some(beta);
            }
        }
        None
    }

    /// Calculates the Late Move Reduction (LMR) for a given move.
    pub(super) fn calculate_reduction(&self, mv: Move, depth: i32, moves_played: i32) -> i32 {
        if mv.is_quiet() && moves_played >= LMR_MOVES_PLAYED && depth >= LMR_DEPTH {
            let mut reduction = (LMR_BASE + f64::from(depth).ln() * f64::from(moves_played).ln() / LMR_DIVISOR) as i32;
            // Adjust reduction based on history heuristic
            reduction -= self.history.get(mv) / LMR_HISTORY_DIVISOR;
            // Avoid negative reductions
            reduction.clamp(0, depth)
        } else {
            0
        }
    }
}

/// If enough quiet moves have been searched at a low depth, it's unlikely that
/// the remaining moves that are ordered later in move list are going to be better.
pub(super) fn quiet_late_move_pruning(depth: i32, quiets_played: i32) -> bool {
    depth <= QLMP_DEPTH && quiets_played > QLMP_QUIETS_PLAYED + depth * depth
}

/// If the static evaluation is significantly lower than alpha at low depths,
/// it's unlikely that the remaining depth will be sufficient to correct
/// the position and raise score above alpha, as is the case with later moves.
pub(super) fn futility_pruning(depth: i32, alpha: i32, eval: i32) -> bool {
    depth <= FUTILITY_DEPTH && eval + FUTILITY_MARGIN * depth + FUTILITY_FIXED_MARGIN < alpha
}
