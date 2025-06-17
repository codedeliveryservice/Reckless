use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::{
    board::Board,
    evaluate::evaluate,
    search::{self, Report},
    tb::tb_initilize,
    thread::{ThreadData, ThreadPool},
    time::{Limits, TimeManager},
    tools,
    transposition::{TranspositionTable, DEFAULT_TT_SIZE},
    types::{is_decisive, is_loss, is_win, Color},
};

pub fn message_loop() {
    static STOP: AtomicBool = AtomicBool::new(false);

    let tt = TranspositionTable::default();
    let repeat_depth = AtomicBool::new(false);
    let counter = AtomicU64::new(0);
    let tb_hits = AtomicU64::new(0);

    let mut threads = ThreadPool::new(&tt, &STOP, &repeat_depth, &counter, &tb_hits);
    for thread in threads.iter_mut() {
        thread.nnue.full_refresh(&thread.board);
    }

    let mut move_overhead = 0;
    let mut report = Report::Full;
    let mut next_command = None;

    loop {
        let command = next_command.take().unwrap_or_else(read_stdin);
        let tokens = command.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["uci"] => uci(),
            ["isready"] => println!("readyok"),

            ["go", tokens @ ..] => next_command = go(&mut threads, &STOP, report, move_overhead, tokens),
            ["position", tokens @ ..] => position(&mut threads, tokens),
            ["setoption", tokens @ ..] => set_option(&mut threads, &mut report, &mut move_overhead, &tt, tokens),
            ["ucinewgame"] => reset(&mut threads, &tt),

            ["stop"] => STOP.store(true, Ordering::Relaxed),
            ["quit"] => break,

            // Non-UCI commands
            ["compiler"] => compiler(),
            ["eval"] => eval(threads.main_thread()),
            ["d"] => display(threads.main_thread()),
            ["bench", v @ ..] => tools::bench::<true>(v.first().and_then(|v| v.parse().ok())),
            ["perft", depth] => tools::perft(depth.parse().unwrap(), &mut threads.main_thread().board),
            ["perft"] => eprintln!("Usage: perft <depth>"),

            _ => eprintln!("Unknown command: '{}'", command.trim_end()),
        };
    }
}

fn uci() {
    println!("id name Reckless {}", env!("CARGO_PKG_VERSION"));
    println!("id author Arseniy Surkov, Shahin M. Shahin, and Styx");
    println!("option name Hash type spin default {DEFAULT_TT_SIZE} min 1 max 262144");
    println!("option name Threads type spin default 1 min 1 max 512");
    println!("option name MoveOverhead type spin default 0 min 0 max 2000");
    println!("option name Minimal type check default false");
    println!("option name Clear Hash type button");
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

fn reset(threads: &mut ThreadPool, tt: &TranspositionTable) {
    threads.clear();
    tt.clear(threads.len());
}

fn go(
    threads: &mut ThreadPool, stop: &'static AtomicBool, report: Report, move_overhead: u64, tokens: &[&str],
) -> Option<String> {
    let board = &threads.main_thread().board;
    let limits = parse_limits(board.side_to_move(), tokens);

    threads.main_thread().time_manager = TimeManager::new(limits, board.fullmove_number(), move_overhead);
    threads.main_thread().tt.increment_age();

    stop.store(false, Ordering::Relaxed);

    let listener = std::thread::scope(|scope| {
        let mut handlers = Vec::new();

        for (id, td) in threads.iter_mut().enumerate() {
            let handler = scope.spawn(move || {
                search::start(td, if id == 0 { report } else { Report::None });
                stop.store(true, Ordering::Relaxed);
            });

            handlers.push(handler);
        }

        let listener = std::thread::spawn(|| loop {
            let command = read_stdin();
            match command.as_str().trim() {
                "isready" => println!("readyok"),
                "stop" => {
                    stop.store(true, Ordering::Relaxed);
                    return None;
                }
                _ => return Some(command),
            }
        });

        for handler in handlers {
            handler.join().unwrap();
        }
        listener
    });

    let min_score = threads.iter().map(|v| v.best_score).min().unwrap();
    let vote_value = |td: &ThreadData| (td.best_score - min_score + 10) * td.completed_depth;

    let mut votes = vec![0; 4096];
    for result in threads.iter() {
        votes[result.pv.best_move().encoded()] += vote_value(result);
    }

    let mut best = 0;

    for current in 1..threads.len() {
        let is_better_candidate = || -> bool {
            let best = &threads[best];
            let current = &threads[current];

            if is_decisive(best.best_score) {
                return current.best_score > best.best_score;
            }

            if is_win(current.best_score) {
                return true;
            }

            let best_vote = votes[best.pv.best_move().encoded()];
            let current_vote = votes[current.pv.best_move().encoded()];

            !is_loss(current.best_score)
                && (current_vote > best_vote || (current_vote == best_vote && vote_value(current) > vote_value(best)))
        };

        if is_better_candidate() {
            best = current;
        }
    }

    println!("bestmove {}", threads[best].pv.best_move());
    crate::misc::dbg_print();

    listener.join().unwrap()
}

fn position(threads: &mut ThreadPool, mut tokens: &[&str]) {
    let mut board = Board::default();

    while !tokens.is_empty() {
        match tokens {
            ["startpos", rest @ ..] => {
                board = Board::starting_position();
                tokens = rest;
            }
            ["fen", rest @ ..] => {
                match Board::new(&rest.join(" ")) {
                    Ok(b) => board = b,
                    Err(e) => eprintln!("Invalid FEN: {e:?}"),
                }
                tokens = rest;
            }
            ["moves", rest @ ..] => {
                rest.iter().for_each(|uci_move| make_uci_move(&mut board, uci_move));
                break;
            }
            _ => {
                tokens = &tokens[1..];
                continue;
            }
        }
    }

    for thread in threads.iter_mut() {
        thread.board = board.clone();
        thread.nnue.full_refresh(&thread.board);
    }
}

fn make_uci_move(board: &mut Board, uci_move: &str) {
    let moves = board.generate_all_moves();
    if let Some(mv) = moves.iter().map(|entry| entry.mv).find(|mv| mv.to_string() == uci_move) {
        board.make_move(mv);
        board.increment_game_ply();
    }
}

fn set_option(
    threads: &mut ThreadPool, report: &mut Report, move_overhead: &mut u64, tt: &TranspositionTable, tokens: &[&str],
) {
    match tokens {
        ["name", "Minimal", "value", v] => match *v {
            "true" => *report = Report::Minimal,
            "false" => *report = Report::Full,
            _ => eprintln!("Invalid value: '{v}'"),
        },
        ["name", "Clear", "Hash"] => {
            tt.clear(threads.len());
            println!("info string Hash cleared");
        }
        ["name", "Hash", "value", v] => {
            tt.resize(threads.len(), v.parse().unwrap());
            println!("info string set Hash to {v} MB");
        }
        ["name", "Threads", "value", v] => {
            threads.set_count(v.parse().unwrap());
            println!("info string set Threads to {v}");
        }
        ["name", "MoveOverhead", "value", v] => {
            *move_overhead = v.parse().unwrap();
            println!("info string set MoveOverhead to {v} ms");
        }
        ["name", "SyzygyPath", "value", v] => match tb_initilize(v) {
            Some(size) => println!("info string Loaded Syzygy tablebases with {size} pieces"),
            None => eprintln!("Failed to load Syzygy tablebases"),
        },
        #[cfg(feature = "spsa")]
        ["name", name, "value", v] => {
            crate::parameters::set_parameter(name, v);
            println!("info string set {name} to {v}");
        }
        _ => eprintln!("Unknown option: '{}'", tokens.join(" ").trim_end()),
    }
}

fn eval(td: &mut ThreadData) {
    let eval = match td.board.side_to_move() {
        Color::White => evaluate(td),
        Color::Black => -evaluate(td),
    };
    println!("{eval}");
}

fn display(td: &mut ThreadData) {
    println!("FEN: {}", td.board.to_fen());
}

fn read_stdin() -> String {
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).unwrap();
    buf
}

fn parse_limits(color: Color, tokens: &[&str]) -> Limits {
    if let ["infinite"] = tokens {
        return Limits::Infinite;
    }

    let mut main = None;
    let mut inc = None;
    let mut moves = None;

    for chunk in tokens.chunks(2) {
        if let [name, value] = *chunk {
            let Ok(value) = value.parse() else {
                continue;
            };

            match name {
                "depth" if value > 0 => return Limits::Depth(value),
                "movetime" if value > 0 => return Limits::Time(value as u64),
                "nodes" if value > 0 => return Limits::Nodes(value as u64),

                "wtime" if Color::White == color => main = Some(value),
                "btime" if Color::Black == color => main = Some(value),
                "winc" if Color::White == color => inc = Some(value),
                "binc" if Color::Black == color => inc = Some(value),
                "movestogo" => moves = Some(value as u64),

                _ => continue,
            }
        }
    }

    if main.is_none() && inc.is_none() {
        return Limits::Infinite;
    }

    let main = main.unwrap_or_default().max(0) as u64;
    let inc = inc.unwrap_or_default().max(0) as u64;

    match moves {
        Some(moves) => Limits::Cyclic(main, inc, moves),
        None => Limits::Fischer(main, inc),
    }
}
