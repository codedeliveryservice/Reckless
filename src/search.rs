use std::time::Instant;

use crate::{
    board::Board,
    thread::ThreadData,
    types::{Score, MAX_PLY},
};

const PV: bool = true;
const NON_PV: bool = false;

pub fn start(td: &mut ThreadData, silent: bool) {
    let now = Instant::now();

    for depth in 1..MAX_PLY as i32 {
        let score = search::<PV>(td, -Score::INFINITE, Score::INFINITE, depth);

        if td.stopped {
            break;
        }

        if !silent {
            td.print_uci_info(depth, score, now);
        }

        td.completed_depth = depth;

        if td.time_manager.soft_limit(depth, td.nodes) {
            break;
        }
    }
}

fn search<const PV: bool>(td: &mut ThreadData, alpha: i32, beta: i32, depth: i32) -> i32 {
    0
}
