use std::{sync::Arc, time::Instant};

use crate::{
    board::{Board, NullBoardObserver},
    search::Report,
    thread::{SharedContext, Status},
    threadpool::ThreadPool,
    time::{Limits, TimeManager},
    types::Color,
};

/// Fixed game continuation used by `speedtest`.
struct Game {
    /// Number of opening moves replayed only to seed the position.
    opening_plies: usize,

    /// Full move list: opening seed followed by measured positions.
    moves: &'static [&'static str],
}

/// Five deterministic Reckless self-play continuations used by `speedtest`.
///
/// The continuations were generated with:
///
/// - UCI options `Threads=1` and `Hash=16`
/// - the first `opening_plies` moves as the fixed opening seed
/// - repeated `go nodes 50000`, appending each returned `bestmove`
///
/// The corpus is stored as moves rather than FENs to save space. Move replay is
/// outside the timed search section.
const GAMES: &[Game] = &[
    Game {
        opening_plies: 4,
        moves: &[
            "e2e4", "c7c5", "g1f3", "d7d6", "f1b5", "c8d7", "b5d7", "b8d7", "c2c3", "g8f6", "d1e2", "e7e6", "e1g1",
            "f8e7", "d2d4", "c5d4", "c3d4", "e8g8", "b1c3", "d8a5", "c1d2", "a5a6", "e2a6", "b7a6", "e4e5", "d6e5",
            "d4e5", "f6g4", "d2f4", "f8b8", "h2h3", "g4h6", "a1d1", "d7f8", "f4h6", "g7h6", "b2b3", "f8g6", "d1d4",
            "a6a5", "c3e4", "b8b5", "e4f6", "g8g7", "f6h5", "g7f8", "f1e1", "b5c5", "e1e2", "a8d8", "d4e4", "d8d3",
            "h5f6", "e7d8", "f6h7", "f8g7",
        ],
    },
    Game {
        opening_plies: 4,
        moves: &[
            "d2d4", "g8f6", "c2c4", "e7e6", "g1f3", "b7b6", "a2a3", "c8b7", "b1c3", "d7d5", "d1c2", "c7c5", "c4d5",
            "c5d4", "f3d4", "f6d5", "d4b5", "d5c3", "c2c3", "b7c6", "b5d4", "c6b7", "d4b5", "b7c6", "c3e5", "c6b5",
            "e5b5", "d8d7", "b5d7", "b8d7", "c1f4", "f8e7", "e2e4", "g7g5", "f4e3", "e7f6", "e1c1", "e8e7", "c1b1",
            "h8d8", "f1b5", "d7c5", "d1c1", "a8c8", "e4e5", "f6e5", "e3g5", "e5f6", "g5h6", "c5b3", "c1c8", "d8c8",
            "h1d1", "b3d4", "b5d3", "c8d8",
        ],
    },
    Game {
        opening_plies: 4,
        moves: &[
            "c2c4", "e7e5", "g1f3", "b8c6", "d2d4", "e5d4", "f3d4", "g8f6", "b1c3", "f8b4", "d4c6", "d7c6", "d1d8",
            "e8d8", "c1d2", "c8e6", "e2e4", "a7a5", "a2a3", "b4c5", "f1e2", "f6d7", "c3a4", "f7f6", "a4c5", "d7c5",
            "e4e5", "f6e5", "d2c3", "c5b3", "a1d1", "b3d4", "e1g1", "d8e7", "e2d3", "c6c5", "d1e1", "e7d6", "f2f4",
            "h8f8", "f4e5", "d6e7", "f1f8", "a8f8", "c3a5", "b7b6", "a5c3", "f8d8", "e1e3", "h7h6", "e3g3", "g7g5",
            "h2h3", "d8f8", "g3e3", "h6h5",
        ],
    },
    Game {
        opening_plies: 4,
        moves: &[
            "g1f3", "d7d5", "d2d4", "g8f6", "c2c4", "e7e6", "g2g3", "d5c4", "f1g2", "f8b4", "c1d2", "c7c5", "a2a3",
            "b4d2", "b1d2", "c5d4", "d2c4", "b8c6", "e1g1", "e8g8", "a1c1", "a8b8", "b2b4", "a7a6", "f3e5", "c6e5",
            "c4e5", "d8b6", "d1d3", "c8d7", "e5d7", "f6d7", "f1d1", "e6e5", "e2e3", "f8d8", "e3d4", "d7f6", "d4d5",
            "b6d6", "d3e3", "d8d7", "e3c5", "g7g6", "c5d6", "d7d6", "d1e1", "e5e4", "g2e4", "f6e4", "e1e4", "d6d5",
            "c1c7", "d5f5", "g1g2", "f5f6",
        ],
    },
    Game {
        opening_plies: 6,
        moves: &[
            "e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6", "b5a4", "g8f6", "e1g1", "f8e7", "f1e1", "b7b5", "a4b3",
            "d7d6", "c2c3", "e8g8", "h2h3", "c8b7", "d2d4", "f8e8", "b1d2", "e7f8", "a2a3", "h7h6", "b3a2", "d8d7",
            "a3a4", "g7g6", "d4d5", "c6e7", "f3h2", "f8g7", "h2g4", "f6h5", "d2b3", "g8h7", "a2b1", "e8f8", "g4e3",
            "h7h8", "c3c4", "b5c4", "b3a5", "b7c8", "a1a3", "a8b8", "a5c4", "h5f4", "e3c2", "f4h5", "c1d2", "f7f5",
            "c2b4", "f5e4", "b1e4", "h5f6", "f2f3", "e7f5",
        ],
    },
];

const DEFAULT_TT_PER_THREAD: usize = 128;
const DEFAULT_DURATION_S: u64 = 150;
const WARMUP_POSITIONS: usize = 3;

/// Run a timed throughput benchmark over fixed Reckless self-play continuations.
///
/// The benchmark warms up on the first three measured positions, clears state, then replays the
/// full corpus. Search state and hash are reused within a game and cleared between games.
pub fn speedtest<const PRETTY: bool>(args: &[&str]) {
    let threads = args.first().and_then(|v| v.parse().ok()).unwrap_or_else(hardware_threads);
    let hash = args.get(1).and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_TT_PER_THREAD * threads);
    let seconds = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_DURATION_S);
    let available_threads = hardware_threads();

    let shared = Arc::new(SharedContext::default());
    let mut pool = ThreadPool::new(shared.clone());

    pool.set_count(threads);
    shared.tt.resize(pool.len(), hash);

    let scale = seconds as f64 * 1000.0 / base_suite_ms();

    run_positions::<false>(&mut pool, &shared, scale, WARMUP_POSITIONS);
    search_clear(&mut pool, &shared);

    if PRETTY {
        println!("{}", "-".repeat(72));
        println!("{:>8} {:>15} {:>12} {:>15} {:>8}", "Position", "Nodes", "Elapsed", "NPS", "Hash");
        println!("{}", "-".repeat(72));
    }

    let results = run_positions::<PRETTY>(&mut pool, &shared, scale, usize::MAX);
    let elapsed_ms = results.elapsed_ms.max(1);
    let nps = 1000.0 * results.nodes as f64 / elapsed_ms as f64;

    if PRETTY {
        println!("{}", "-".repeat(72));
        println!("Reckless {}", env!("ENGINE_VERSION"));
        println!("Command: {}", render_invocation(args));
        println!("Available processors: {available_threads}");
        println!("Threads: {}", pool.len());
        println!("Hash: {hash} MiB");
        println!("Target duration: {seconds} s");
        println!("Total nodes searched: {}", results.nodes);
        println!("Total search time (ms): {elapsed_ms}");
        println!("Nodes/second: {nps:.0}");
        println!("Hashfull max/avg [per mille]: {} / {:.1}", results.max_hash, results.avg_hash());
    } else {
        println!("Speedtest: {} nodes {nps:.0} nps", results.nodes);
    }

    crate::misc::dbg_print();
}

#[derive(Default)]
struct Results {
    positions: usize,
    nodes: u64,
    elapsed_ms: u128,
    hash_total: usize,
    max_hash: usize,
}

impl Results {
    fn add(&mut self, nodes: u64, elapsed_ms: u128, hash: usize) {
        self.positions += 1;
        self.nodes += nodes;
        self.elapsed_ms += elapsed_ms;
        self.hash_total += hash;
        self.max_hash = self.max_hash.max(hash);
    }

    fn avg_hash(&self) -> f64 {
        if self.positions == 0 { 0.0 } else { self.hash_total as f64 / self.positions as f64 }
    }
}

fn run_positions<const PRETTY: bool>(
    pool: &mut ThreadPool, shared: &Arc<SharedContext>, scale: f64, limit: usize,
) -> Results {
    let mut results = Results::default();

    'suite: for game in GAMES {
        search_clear(pool, shared);
        let mut board = Board::starting_position();

        for (index, &mv) in game.moves.iter().enumerate() {
            if index < game.opening_plies {
                make_uci_move(&mut board, mv);
                continue;
            }

            if results.positions == limit {
                break 'suite;
            }

            make_uci_move(&mut board, mv);

            let ply = index + 1 - game.opening_plies;
            let movetime = (base_time_ms(ply) * scale) as u64;
            let start = Instant::now();

            pool.execute_searches(TimeManager::new(Limits::Time(movetime), 0, 0), Report::None, 1, &board, shared);

            let elapsed_ms = start.elapsed().as_millis();
            let nodes = shared.nodes.aggregate();
            let hash = shared.tt.hashfull();
            results.add(nodes, elapsed_ms, hash);

            if PRETTY {
                let nps = 1000.0 * nodes as f64 / elapsed_ms.max(1) as f64;
                println!("{:>8} {:>15} {:>9} ms {:>15.0} {:>8}", results.positions, nodes, elapsed_ms, nps, hash);
            }
        }
    }

    results
}

/// Apply a UCI move like `uci::make_uci_move`, but panic on invalid corpus data.
fn make_uci_move(board: &mut Board, uci_move: &str) {
    let mv = board
        .generate_all_moves()
        .iter()
        .map(|entry| entry.mv)
        .find(|mv| mv.to_uci(board) == uci_move)
        .unwrap_or_else(|| panic!("invalid speedtest move {uci_move} in {}", board.to_fen()));

    board.make_move(mv, &mut NullBoardObserver);
    board.advance_fullmove_counter();
}

fn search_clear(pool: &mut ThreadPool, shared: &Arc<SharedContext>) {
    pool.clear();
    shared.tt.clear(pool.len());

    for corrhist in shared.history.all() {
        corrhist.pawn.clear();
        corrhist.non_pawn[Color::White].clear();
        corrhist.non_pawn[Color::Black].clear();
    }

    shared.status.set(Status::STOPPED);
}

fn hardware_threads() -> usize {
    std::thread::available_parallelism().map_or(1, |threads| threads.get())
}

fn render_invocation(args: &[&str]) -> String {
    if args.is_empty() { "speedtest".to_string() } else { format!("speedtest {}", args.join(" ")) }
}

fn base_suite_ms() -> f64 {
    GAMES
        .iter()
        .map(|game| {
            let positions = game.moves.len() - game.opening_plies;
            (1..=positions).map(base_time_ms).sum::<f64>()
        })
        .sum()
}

fn base_time_ms(ply: usize) -> f64 {
    50000.0 / (ply as f64 + 15.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speedtest_corpus_moves_are_legal() {
        crate::lookup::initialize();

        for game in GAMES {
            let mut board = Board::starting_position();
            for &mv in game.moves {
                make_uci_move(&mut board, mv);
            }
        }
    }
}
