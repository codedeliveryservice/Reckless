use crate::types::{Move, Score};

const RFP_MARGIN: i32 = 75;
const RFP_DEPTH: i32 = 8;

const NMP_DEPTH: i32 = 3;
const NMP_REDUCTION: i32 = 3;
const NMP_DIVISOR: i32 = 4;

const RAZORING_DEPTH: i32 = 2;
const RAZORING_MARGIN: i32 = 200;
const RAZORING_FIXED_MARGIN: i32 = 125;

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

impl super::Searcher<'_> {
    /// If the static evaluation of the position is significantly higher than beta
    /// at low depths, it's likely to be good enough to cause a beta cutoff.
    pub fn reverse_futility_pruning(&self, depth: i32, beta: i32, eval: i32, improving: bool) -> Option<i32> {
        if depth < RFP_DEPTH && eval - RFP_MARGIN * (depth - i32::from(improving)) > beta {
            return Some(eval);
        }
        None
    }

    /// If giving a free move to the opponent leads to a beta cutoff, it's highly likely
    /// to result in a cutoff after a real move is made, so the node can be pruned.
    pub fn null_move_pruning<const PV: bool>(&mut self, depth: i32, beta: i32, eval: i32) -> Option<i32> {
        if depth >= NMP_DEPTH && eval > beta && !self.board.is_last_move_null() && self.board.has_non_pawn_material() {
            let r = NMP_REDUCTION + depth / NMP_DIVISOR + ((eval - beta) / 200).min(3);

            self.board.make_null_move();
            let score = -self.alpha_beta::<PV, false>(-beta, -beta + 1, depth - r);
            self.board.undo_move::<false>();

            // Avoid returning false mates
            if score >= Score::MATE_BOUND {
                return Some(beta);
            }

            if score >= beta {
                return Some(score);
            }
        }
        None
    }

    /// If the static evaluation of the position is significantly lower than alpha, return
    /// the result of a quiescence search since the node is likely to fail low anyway.
    pub fn razoring(&mut self, depth: i32, alpha: i32, beta: i32, eval: i32) -> Option<i32> {
        if depth <= RAZORING_DEPTH && eval + RAZORING_MARGIN * depth + RAZORING_FIXED_MARGIN < alpha {
            return Some(self.quiescence_search(alpha, beta));
        }
        None
    }

    /// Calculates the Late Move Reduction (LMR) for a given move.
    pub fn calculate_reduction<const PV: bool>(&self, mv: Move, depth: i32, moves_played: i32) -> i32 {
        if mv.is_quiet() && moves_played >= LMR_MOVES_PLAYED && depth >= LMR_DEPTH {
            let mut reduction = (LMR_BASE + f64::from(depth).ln() * f64::from(moves_played).ln() / LMR_DIVISOR) as i32;
            // Adjust reduction based on history heuristic
            reduction -= self.history.get(mv) / LMR_HISTORY_DIVISOR;
            // Reduce PV nodes less
            reduction -= i32::from(PV);
            // Reduce checks less
            reduction -= i32::from(self.board.is_in_check());
            // Avoid negative reductions
            reduction.clamp(0, depth)
        } else {
            0
        }
    }
}

/// If enough quiet moves have been searched at a low depth, it's unlikely that
/// the remaining moves that are ordered later in move list are going to be better.
pub fn quiet_late_move_pruning(depth: i32, quiets_played: i32, improving: bool) -> bool {
    depth <= QLMP_DEPTH && quiets_played > QLMP_QUIETS_PLAYED + depth * depth / (1 + i32::from(!improving))
}

/// If the static evaluation is significantly lower than alpha at low depths,
/// it's unlikely that the remaining depth will be sufficient to correct
/// the position and raise score above alpha, as is the case with later moves.
pub fn futility_pruning(depth: i32, alpha: i32, eval: i32) -> bool {
    depth <= FUTILITY_DEPTH && eval + FUTILITY_MARGIN * depth + FUTILITY_FIXED_MARGIN < alpha
}
