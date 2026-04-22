use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use crate::{
    board::{Board, NullBoardObserver},
    search::Report,
    thread::{SharedContext, Status, ThreadData},
    threadpool::ThreadPool,
    time::{Limits, TimeManager},
    tools,
    transposition::DEFAULT_TT_SIZE,
    types::{Color, MAX_MOVES, Move, Piece, Score, Square, is_decisive, is_loss, is_win},
};

#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Cli,
    Uci,
}

struct Settings {
    frc: bool,
    ponder: bool,
    multi_pv: usize,
    move_overhead: u64,
    report: Report,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            frc: false,
            ponder: false,
            multi_pv: 1,
            move_overhead: 100,
            report: Report::Full,
        }
    }
}

pub fn message_loop(mut buffer: VecDeque<String>) {
    let shared = Arc::new(SharedContext::default());
    let mut settings = Settings::default();
    let mut threads = ThreadPool::new(shared.clone());

    let rx = spawn_listener(shared.clone());

    let mut mode = if buffer.is_empty() { Mode::Uci } else { Mode::Cli };

    loop {
        let message = if let Some(cmd) = buffer.pop_front() {
            cmd
        } else if mode == Mode::Uci {
            match rx.recv() {
                Ok(cmd) => cmd,
                Err(_) => break,
            }
        } else {
            break;
        };

        let tokens = message.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["uci"] => {
                uci();
                mode = Mode::Uci;
            }

            ["isready"] => println!("readyok"),

            ["go", tokens @ ..] => go(&mut threads, &settings, &shared, tokens),
            ["position", tokens @ ..] => position(&mut threads, &settings, tokens),
            ["setoption", tokens @ ..] => set_option(&mut threads, &mut settings, &shared, tokens),
            ["ucinewgame"] => reset(&mut threads, &shared),

            ["stop"] => shared.status.set(Status::STOPPED),
            ["quit"] => {
                drop(threads);
                break;
            }

            // Non-UCI commands
            ["compiler"] => compiler(),
            ["eval"] => eval(threads.main_thread()),
            ["d"] => println!("{}", threads.main_thread().board),
            ["bench", args @ ..] => match mode {
                Mode::Uci => tools::bench::<true>(args),
                Mode::Cli => tools::bench::<false>(args),
            },
            ["perft", depth] => tools::perft(depth.parse().unwrap(), &mut threads.main_thread().board),
            ["perft"] => eprintln!("Usage: perft <depth>"),
            ["simpleperft", depth] => tools::simple_perft(depth.parse().unwrap(), &mut threads.main_thread().board),
            ["simpleperft"] => eprintln!("Usage: simpleperft <depth>"),
            ["islegalperft", depth] => tools::is_legal_perft(depth.parse().unwrap(), &mut threads.main_thread().board),
            ["islegalperft"] => eprintln!("Usage: islegalperft <depth>"),

            // Ignore empty lines
            [] => (),

            _ => eprintln!("Unknown command: '{}'", message.trim_end()),
        }

        // Auto-exit after last CLI command
        if matches!(mode, Mode::Cli) && buffer.is_empty() {
            drop(threads);
            break;
        }
    }
}

fn spawn_listener(shared: Arc<SharedContext>) -> std::sync::mpsc::Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        loop {
            let mut message = String::new();

            if std::io::stdin().read_line(&mut message).unwrap() == 0 {
                // EOF received
                if shared.status.get() != Status::RUNNING {
                    let _ = tx.send("quit".to_string());
                }
            }

            match message.trim_end() {
                "isready" => println!("readyok"),
                "stop" => shared.status.set(Status::STOPPED),
                "ponderhit" => {
                    shared.ponderhit.store(true, Ordering::Release);
                }
                "quit" => {
                    shared.status.set(Status::STOPPED);
                    let _ = tx.send("quit".to_string());
                    break;
                }
                _ => {
                    // According to the UCI specs, commands that are unexpected
                    // in the current state should be ignored silently.
                    // (https://backscattering.de/chess/uci/#unexpected)
                    if shared.status.get() != Status::RUNNING {
                        let _ = tx.send(message);
                    }
                }
            }
        }
    });

    rx
}

fn uci() {
    println!("id name Reckless {}", env!("ENGINE_VERSION"));
    println!("id author Arseniy Surkov, Shahin M. Shahin, and Styx");
    println!("option name Hash type spin default {DEFAULT_TT_SIZE} min 1 max 262144");
    println!("option name Threads type spin default 1 min 1 max {}", ThreadPool::available_threads());
    println!("option name MoveOverhead type spin default 100 min 0 max 2000");
    println!("option name Minimal type check default false");
    println!("option name Clear Hash type button");
    println!("option name UCI_Chess960 type check default false");
    println!("option name Ponder type check default false");
    println!("option name MultiPV type spin default 1 min 1 max {MAX_MOVES}");

    #[cfg(feature = "syzygy")]
    println!("option name SyzygyPath type string default");

    #[cfg(feature = "spsa")]
    crate::parameters::print_options();

    println!("uciok");
}

fn compiler() {
    println!("Compiler Version: {}", env!("COMPILER_VERSION"));
    println!("Compiler Target: {}", env!("COMPILER_TARGET"));
    println!("Compiler Features: {}", env!("COMPILER_FEATURES"));
}

fn reset(threads: &mut ThreadPool, shared: &Arc<SharedContext>) {
    threads.clear();
    shared.tt.clear(threads.len());

    for corrhist in unsafe { shared.replicator.get_all() } {
        corrhist.pawn.clear();
        corrhist.non_pawn[Color::White].clear();
        corrhist.non_pawn[Color::Black].clear();
    }
}

fn go(threads: &mut ThreadPool, settings: &Settings, shared: &Arc<SharedContext>, tokens: &[&str]) {
    let board = &threads.main_thread().board;
    let go_options = parse_go_options(board.side_to_move(), tokens);
    let is_ponder = go_options.ponder;
    let time_manager = TimeManager::new(go_options.limits, board.fullmove_number(), settings.move_overhead, is_ponder);

    threads.main_thread().multi_pv = settings.multi_pv;
    threads.execute_searches(time_manager, settings.report, shared, is_ponder);

    let min_score = threads.iter().map(|v| v.root_moves[0].score).min().unwrap();
    let vote_value = |td: &ThreadData| (td.root_moves[0].score - min_score + 10) * td.completed_depth;

    let mut votes: HashMap<&Move, i32> = HashMap::new();
    for result in threads.iter() {
        *votes.entry(&result.root_moves[0].mv).or_default() += vote_value(result);
    }

    let mut best = 0;

    if !matches!(threads[best].time_manager.limits(), Limits::Depth(_)) && threads[0].multi_pv == 1 {
        for current in 1..threads.len() {
            let is_better_candidate = || -> bool {
                let best = &threads[best];
                let current = &threads[current];

                if is_win(best.root_moves[0].score) {
                    return current.root_moves[0].score > best.root_moves[0].score;
                }

                if current.root_moves[0].score != -Score::INFINITE
                    && best.root_moves[0].score != -Score::INFINITE
                    && is_loss(best.root_moves[0].score)
                {
                    return current.root_moves[0].score < best.root_moves[0].score;
                }

                if current.root_moves[0].score != -Score::INFINITE && is_decisive(current.root_moves[0].score) {
                    return true;
                }

                let best_vote = votes[&best.root_moves[0].mv];
                let current_vote = votes[&current.root_moves[0].mv];

                !is_loss(current.root_moves[0].score)
                    && (current_vote > best_vote
                        || (current_vote == best_vote && vote_value(current) > vote_value(best)))
            };

            if is_better_candidate() {
                best = current;
            }
        }
    }

    if best != 0 {
        threads[best].print_uci_info(threads[best].completed_depth);
    }

    let best_move = threads[best].root_moves[0].mv;
    let mut bestmove_output = format!("bestmove {}", best_move.to_uci(&threads.main_thread().board));

    if let Some(ponder_move) = extract_ponder_move(threads, best, best_move) {
        bestmove_output.push_str(" ponder ");
        bestmove_output.push_str(&ponder_move.to_uci(&threads.main_thread().board));
    }

    println!("{bestmove_output}");
    crate::misc::dbg_print();
}

fn position(threads: &mut ThreadPool, settings: &Settings, mut tokens: &[&str]) {
    let mut board = threads.main_thread().board.clone();

    while !tokens.is_empty() {
        match tokens {
            ["startpos", rest @ ..] => {
                board = Board::starting_position();
                tokens = rest;
            }
            ["fen", rest @ ..] => {
                match Board::from_fen(&rest.join(" ")) {
                    Ok(b) => board = b,
                    Err(e) => eprintln!("Invalid FEN: {e:?}"),
                }
                board.set_frc(settings.frc);
                tokens = rest;
            }
            ["moves", rest @ ..] => {
                for uci_move in rest {
                    make_uci_move(&mut board, uci_move);
                }
                break;
            }
            _ => tokens = &tokens[1..],
        }
    }

    for thread in threads.iter_mut() {
        thread.board = board.clone();
    }
}

fn make_uci_move(board: &mut Board, uci_move: &str) {
    let moves = board.generate_all_moves();
    if let Some(mv) = moves.iter().map(|entry| entry.mv).find(|mv| mv.to_uci(board) == uci_move) {
        board.make_move(mv, &mut NullBoardObserver);
        board.advance_fullmove_counter();
    }
}

fn extract_ponder_move(threads: &mut ThreadPool, best: usize, best_move: Move) -> Option<Move> {
    let root_move = &threads[best].root_moves[0];

    if let Some(&ponder_move) = root_move.pv.line().first() {
        return Some(ponder_move);
    }

    let mut board = threads.main_thread().board.clone();
    board.make_move(best_move, &mut NullBoardObserver);

    let hash = board.hash();
    let halfmove_clock = board.halfmove_clock();
    let tt_entry = threads.main_thread().shared.tt.read(hash, halfmove_clock, 0)?;

    if tt_entry.mv.is_null() {
        return None;
    }

    let is_legal = board.generate_all_moves().iter().any(|entry| entry.mv == tt_entry.mv);
    is_legal.then_some(tt_entry.mv)
}

fn set_option(threads: &mut ThreadPool, settings: &mut Settings, shared: &Arc<SharedContext>, tokens: &[&str]) {
    match tokens {
        ["name", "Minimal", "value", v] => match *v {
            "true" => settings.report = Report::Minimal,
            "false" => settings.report = Report::Full,
            _ => eprintln!("Invalid value: '{v}'"),
        },
        ["name", "Clear", "Hash"] => {
            shared.tt.clear(threads.len());
            println!("info string Hash cleared");
        }
        ["name", "Hash", "value", v] => {
            shared.tt.resize(threads.len(), v.parse().unwrap());
            println!("info string set Hash to {v} MB");
        }
        ["name", "Threads", "value", v] => {
            threads.set_count(v.parse().unwrap_or(1));
            println!("info string set Threads to {}", threads.len());
        }
        ["name", "MoveOverhead", "value", v] => {
            settings.move_overhead = v.parse().unwrap();
            println!("info string set MoveOverhead to {v} ms");
        }
        #[cfg(feature = "syzygy")]
        ["name", "SyzygyPath", "value", v] => match crate::tb::initialize(v) {
            Some(size) => println!("info string Loaded Syzygy tablebases with {size} pieces"),
            None => eprintln!("Failed to load Syzygy tablebases"),
        },
        ["name", "UCI_Chess960", "value", v] => {
            settings.frc = v.parse().unwrap_or_default();
            println!("info string set UCI_Chess960 to {v}");
        }
        ["name", "Ponder", "value", v] => {
            settings.ponder = v.parse().unwrap_or_default();
            println!("info string set Ponder to {v}");
        }
        ["name", "MultiPV", "value", v] => {
            settings.multi_pv = v.parse().unwrap_or_default();
            println!("info string set MultiPV to {v}");
        }
        #[cfg(feature = "spsa")]
        ["name", name, "value", v] => {
            crate::parameters::set_parameter(name, v);
            println!("info string set {name} to {v}");
        }
        _ => eprintln!("Unknown option: '{}'", tokens.join(" ").trim_end()),
    }
}

fn eval(td: &mut ThreadData) {
    td.nnue.full_refresh(&td.board);
    td.nnue.evaluate(&td.board);

    let side = td.board.side_to_move();

    println!("NNUE derived piece values");
    println!("+-------+-------+-------+-------+-------+-------+-------+-------+");
    for rank in (0..8).rev() {
        print!("|");
        for file in 0..8 {
            let sq = Square::from_rank_file(rank, file);
            let piece = td.board.piece_on(sq);
            let piece_str = if piece == Piece::None { " ".to_string() } else { piece.to_string() };
            print!("  {:^3}  |", piece_str);
        }
        println!();

        print!("|");
        for file in 0..8 {
            let sq = Square::from_rank_file(rank, file);
            match td.nnue.piece_contribution(&td.board, sq) {
                None => print!("       |"),
                Some(v) => {
                    let val = v as f32 / 100.0;
                    print!("{:+6.2} |", val);
                }
            }
        }
        println!();
        println!("+-------+-------+-------+-------+-------+-------+-------+-------+");
    }

    let used_bucket = crate::nnue::OUTPUT_BUCKETS_LAYOUT[td.board.occupancies().popcount()];

    println!("\nNNUE output buckets (White side)");
    println!("+------------+------------+");
    println!("|   Bucket   |   Total    |");
    println!("+------------+------------+");

    for bucket in 0..8 {
        let raw_score = td.nnue.eval_with_bucket(&td.board, bucket);
        let white_score = if side == Color::White { raw_score } else { -raw_score };
        let total = white_score as f32 / 100.0;

        if bucket == used_bucket {
            println!("|  {:<2}        | {:+7.2}    | <-- this bucket is used", bucket, total);
        } else {
            println!("|  {:<2}        | {:+7.2}    |", bucket, total);
        }
    }
    println!("+------------+------------+");

    let final_eval = td.nnue.evaluate(&td.board);
    let final_total = (if side == Color::White { final_eval } else { -final_eval }) as f32 / 100.0;
    println!("\nNNUE evaluation        {:+.2} (White side)", final_total);
}

struct GoOptions {
    limits: Limits,
    ponder: bool,
}

fn parse_go_options(color: Color, tokens: &[&str]) -> GoOptions {
    let mut ponder = false;
    let mut main = None;
    let mut inc = None;
    let mut moves = None;
    let mut direct_limits = None;

    let mut index = 0;
    while index < tokens.len() {
        match (tokens[index], tokens.get(index + 1).and_then(|v| v.parse::<u64>().ok())) {
            ("infinite", _) => {
                direct_limits = Some(Limits::Infinite);
                index += 1;
            }
            ("ponder", _) => {
                ponder = true;
                index += 1;
            }
            ("depth", Some(value)) if value > 0 => {
                direct_limits = Some(Limits::Depth(value as i32));
                index += 2;
            }
            ("movetime", Some(value)) if value > 0 => {
                direct_limits = Some(Limits::Time(value));
                index += 2;
            }
            ("nodes", Some(value)) if value > 0 => {
                direct_limits = Some(Limits::Nodes(value));
                index += 2;
            }
            ("wtime", Some(value)) if Color::White == color => {
                main = Some(value);
                index += 2;
            }
            ("btime", Some(value)) if Color::Black == color => {
                main = Some(value);
                index += 2;
            }
            ("winc", Some(value)) if Color::White == color => {
                inc = Some(value);
                index += 2;
            }
            ("binc", Some(value)) if Color::Black == color => {
                inc = Some(value);
                index += 2;
            }
            ("movestogo", Some(value)) => {
                moves = Some(value);
                index += 2;
            }
            ("depth" | "movetime" | "nodes" | "wtime" | "btime" | "winc" | "binc" | "movestogo", None) => {
                index += 1;
            }
            _ => {
                index += 1;
            }
        }
    }

    let limits = if let Some(direct_limits) = direct_limits {
        direct_limits
    } else if main.is_none() && inc.is_none() {
        Limits::Infinite
    } else {
        let main = main.unwrap_or_default();
        let inc = inc.unwrap_or_default();

        match moves {
            Some(moves) => Limits::Cyclic(main, inc, moves),
            None => Limits::Fischer(main, inc),
        }
    };

    GoOptions { limits, ponder }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thread::RootMove;
    use crate::transposition::{Bound, TtDepth};

    fn test_position_helper(tokens: &[&str]) -> Board {
        let shared = Arc::new(SharedContext::default());
        let settings = Settings::default();
        let mut threads = ThreadPool::new(shared);

        position(&mut threads, &settings, tokens);
        threads.main_thread().board.clone()
    }

    fn find_move(board: &Board, uci: &str) -> Move {
        board.generate_all_moves().iter().map(|entry| entry.mv).find(|mv| mv.to_uci(board) == uci).unwrap()
    }

    #[test]
    fn test_position_startpos() {
        let board = test_position_helper(&["startpos"]);
        assert_eq!(board.to_fen(), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        let board = test_position_helper(&[]);
        assert_eq!(board.to_fen(), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    }

    #[test]
    fn test_position_startpos_multiple_moves() {
        let board = test_position_helper(&["moves", "e2e4", "e7e5", "g1f3"]);
        assert_eq!(board.side_to_move(), Color::Black);
        let fen = board.to_fen();
        let fen_position = fen.split_whitespace().next().unwrap();
        assert!(fen_position.contains("5N2"));
    }

    #[test]
    fn test_position_fen_with_moves() {
        let board = test_position_helper(&[
            "fen",
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR",
            "b",
            "KQkq",
            "e3",
            "0",
            "1",
            "moves",
            "e7e5",
        ]);
        assert_eq!(board.side_to_move(), Color::White);
    }

    #[test]
    fn test_position_empty_moves_list() {
        let board = test_position_helper(&["moves"]);
        assert_eq!(board.to_fen(), "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    }

    #[test]
    fn test_position_invalid_move_ignored() {
        let board = test_position_helper(&["moves", "e2e4", "invalid", "e7e5"]);
        assert_eq!(board.side_to_move(), Color::White);
    }

    #[test]
    fn test_position_long_move_sequence() {
        let board = test_position_helper(&["moves", "e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "a7a6"]);
        assert_eq!(board.side_to_move(), Color::White);
    }

    #[test]
    fn test_position_castling() {
        let board = test_position_helper(&[
            "fen",
            "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R",
            "w",
            "KQkq",
            "-",
            "0",
            "1",
            "moves",
            "e1g1",
        ]);
        assert_eq!(board.side_to_move(), Color::Black);
    }

    #[test]
    fn test_position_en_passant() {
        let board = test_position_helper(&[
            "fen",
            "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR",
            "w",
            "KQkq",
            "f6",
            "0",
            "1",
            "moves",
            "e5f6",
        ]);
        assert_eq!(board.side_to_move(), Color::Black);
    }

    #[test]
    fn test_position_promotion() {
        let board = test_position_helper(&["fen", "8/P7/8/8/8/8/8/4K2k", "w", "-", "-", "0", "1", "moves", "a7a8q"]);
        assert_eq!(board.side_to_move(), Color::Black);
    }

    #[test]
    fn test_make_uci_move_invalid() {
        let mut board = Board::starting_position();
        let fen_before = board.to_fen();
        make_uci_move(&mut board, "invalid_move");
        assert_eq!(board.to_fen(), fen_before);
    }

    #[test]
    fn test_position_moves_without_startpos_ignored() {
        let board = test_position_helper(&["moves", "e2e4", "e7e5"]);
        assert_eq!(board.to_fen(), "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2");
    }

    #[test]
    fn test_parse_go_options_ponder() {
        let options = parse_go_options(Color::White, &["ponder", "wtime", "1000", "btime", "1000", "winc", "50"]);
        assert!(options.ponder);
        assert!(matches!(options.limits, Limits::Fischer(1000, 50)));
    }

    #[test]
    fn test_parse_go_options_depth() {
        let options = parse_go_options(Color::White, &["depth", "10", "ponder"]);
        assert!(options.ponder);
        assert!(matches!(options.limits, Limits::Depth(10)));
    }

    #[test]
    fn test_parse_go_options_ponder_with_movetime() {
        let options = parse_go_options(Color::White, &["ponder", "movetime", "1000"]);
        assert!(options.ponder);
        assert!(matches!(options.limits, Limits::Time(1000)));
    }

    #[test]
    fn test_parse_go_options_ponder_with_invalid_limit_value() {
        let options = parse_go_options(Color::White, &["wtime", "1000", "winc", "nope", "ponder"]);
        assert!(options.ponder);
        assert!(matches!(options.limits, Limits::Fischer(1000, 0)));
    }

    #[test]
    fn test_extract_ponder_move_prefers_pv() {
        let shared = Arc::new(SharedContext::default());
        let mut threads = ThreadPool::new(shared);

        let best_move = find_move(&threads.main_thread().board, "e2e4");
        let ponder_move = find_move(&threads.main_thread().board, "d2d4");

        let mut pv = crate::thread::PrincipalVariationTable::default();
        pv.update(0, ponder_move);
        threads.main_thread().root_moves = vec![RootMove { mv: best_move, pv, ..Default::default() }];

        assert_eq!(extract_ponder_move(&mut threads, 0, best_move), Some(ponder_move));
    }

    #[test]
    fn test_extract_ponder_move_uses_tt_fallback() {
        let shared = Arc::new(SharedContext::default());
        let mut threads = ThreadPool::new(shared);

        let best_move = find_move(&threads.main_thread().board, "e2e4");
        threads.main_thread().root_moves = vec![RootMove { mv: best_move, ..Default::default() }];

        let mut after_best = threads.main_thread().board.clone();
        after_best.make_move(best_move, &mut NullBoardObserver);

        let reply = find_move(&after_best, "e7e5");
        let hash = after_best.hash();

        threads.main_thread().shared.tt.write(hash, TtDepth::SOME, 0, 0, Bound::Exact, reply, 0, true, false);

        assert_eq!(extract_ponder_move(&mut threads, 0, best_move), Some(reply));
    }

    #[test]
    fn test_extract_ponder_move_rejects_illegal_tt_move() {
        let shared = Arc::new(SharedContext::default());
        let mut threads = ThreadPool::new(shared);

        let best_move = find_move(&threads.main_thread().board, "e2e4");
        threads.main_thread().root_moves = vec![RootMove { mv: best_move, ..Default::default() }];

        let mut after_best = threads.main_thread().board.clone();
        after_best.make_move(best_move, &mut NullBoardObserver);

        // e2e4 is no longer legal for black in the resulting position.
        let illegal_reply = best_move;
        let hash = after_best.hash();

        threads.main_thread().shared.tt.write(hash, TtDepth::SOME, 0, 0, Bound::Exact, illegal_reply, 0, true, false);

        assert_eq!(extract_ponder_move(&mut threads, 0, best_move), None);
    }
}
