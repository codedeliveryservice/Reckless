use std::sync::atomic::{AtomicBool, AtomicU64};

use crate::{
    board::Board,
    evaluate::evaluate,
    search::{self, Report, SearchResult},
    thread::{ThreadData, ThreadPool},
    time::{Limits, TimeManager},
    tools,
    transposition::{TranspositionTable, DEFAULT_TT_SIZE},
    types::{is_decisive, is_loss, is_win, Color},
};

pub fn message_loop() {
    let tt = TranspositionTable::default();
    let stop = AtomicBool::new(false);
    let counter = AtomicU64::new(0);

    let mut report = Report::Full;
    let mut threads = ThreadPool::new(&tt, &stop, &counter);
    for thread in threads.iter_mut() {
        thread.nnue.refresh(&thread.board);
    }

    loop {
        let command = read_stdin();
        let tokens = command.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["uci"] => uci(),
            ["isready"] => println!("readyok"),

            ["go", tokens @ ..] => go(&mut threads, report, tokens),
            ["position", tokens @ ..] => position(&mut threads, tokens),
            ["setoption", tokens @ ..] => set_option(&mut threads, &mut report, &tt, tokens),
            ["ucinewgame"] => reset(&mut threads, &tt),

            ["quit"] => break,

            // Non-UCI commands
            ["eval"] => eval(threads.main_thread()),
            ["bench", depth] => tools::bench::<true>(depth.parse().unwrap()),
            ["perft", depth] => tools::perft(depth.parse().unwrap(), &mut threads.main_thread().board),

            ["bench"] => eprintln!("Usage: bench <depth>"),
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
    println!("option name Minimal type check default false");
    println!("option name Clear Hash type button");

    #[cfg(feature = "spsa")]
    crate::parameters::print_options();

    println!("uciok");
}

fn reset(threads: &mut ThreadPool, tt: &TranspositionTable) {
    threads.clear();
    tt.clear(threads.len());
}

fn go(threads: &mut ThreadPool, report: Report, tokens: &[&str]) {
    let board = &threads.main_thread().board;
    let limits = parse_limits(board.side_to_move(), tokens);

    threads.main_thread().time_manager = TimeManager::new(limits, board.fullmove_number());
    threads.main_thread().set_stop(false);

    let results = std::thread::scope(|scope| {
        let mut handlers = Vec::new();

        for (id, td) in threads.iter_mut().enumerate() {
            let handler = scope.spawn(move || {
                let result = search::start(td, if id == 0 { report } else { Report::None });
                td.set_stop(true);
                result
            });

            handlers.push(handler);
        }

        handlers.into_iter().map(|h| h.join().unwrap()).collect::<Vec<_>>()
    });

    let min_score = results.iter().map(|v| v.score).min().unwrap();
    let vote_value = |result: &SearchResult| (result.score - min_score + 10) * result.depth;

    let mut votes = vec![0; 4096];
    for result in &results {
        votes[result.best_move.encoded()] += vote_value(result);
    }

    let mut best = results[0];
    for current in &results[1..] {
        let is_better_candidate = || -> bool {
            if is_decisive(best.score) {
                return current.score > best.score;
            }
            if is_win(current.score) {
                return true;
            }

            let best_vote = votes[best.best_move.encoded()];
            let current_vote = votes[current.best_move.encoded()];

            !is_loss(current.score)
                && (current_vote > best_vote || (current_vote == best_vote && vote_value(current) > vote_value(&best)))
        };

        if is_better_candidate() {
            best = *current;
        }
    }

    println!("bestmove {}", best.best_move);

    threads.main_thread().tt.increment_age();
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
        thread.nnue.refresh(&thread.board);
    }
}

fn make_uci_move(board: &mut Board, uci_move: &str) {
    let moves = board.generate_all_moves();
    if let Some(mv) = moves.iter().map(|entry| entry.mv).find(|mv| mv.to_string() == uci_move) {
        board.make_move(mv);
        board.increment_game_ply();
    }
}

fn set_option(threads: &mut ThreadPool, report: &mut Report, tt: &TranspositionTable, tokens: &[&str]) {
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

fn read_stdin() -> String {
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).unwrap();
    buf
}

fn parse_limits(color: Color, tokens: &[&str]) -> Limits {
    if let ["infinite"] = tokens {
        return Limits::Infinite;
    }

    let mut main = 0;
    let mut inc = 0;
    let mut moves = None;

    for chunk in tokens.chunks(2) {
        if let [name, value] = *chunk {
            let Ok(value) = value.parse() else {
                continue;
            };

            match name {
                "depth" if value > 0 => return Limits::Depth(value as i32),
                "movetime" if value > 0 => return Limits::Time(value),
                "nodes" if value > 0 => return Limits::Nodes(value),

                "wtime" if Color::White == color => main = value,
                "btime" if Color::Black == color => main = value,
                "winc" if Color::White == color => inc = value,
                "binc" if Color::Black == color => inc = value,
                "movestogo" => moves = Some(value),

                _ => continue,
            }
        }
    }

    if main == 0 && inc == 0 {
        return Limits::Infinite;
    }

    match moves {
        Some(moves) => Limits::Cyclic(main, inc, moves),
        None => Limits::Fischer(main, inc),
    }
}
