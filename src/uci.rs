use std::sync::atomic::AtomicBool;

use crate::{
    board::Board,
    search,
    thread::ThreadPool,
    time::{Limits, TimeManager},
    tools,
    transposition::{TranspositionTable, DEFAULT_TT_SIZE},
    types::Color,
};

pub fn message_loop() {
    let stop = AtomicBool::new(false);
    let tt = TranspositionTable::default();

    let mut threads = ThreadPool::new(&tt, &stop);

    loop {
        let command = read_stdin();
        let tokens = command.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["uci"] => uci(),
            ["isready"] => println!("readyok"),

            ["go", tokens @ ..] => go(&mut threads, tokens),
            ["position", tokens @ ..] => position(&mut threads, tokens),
            ["setoption", tokens @ ..] => set_option(&mut threads, &tt, tokens),
            ["ucinewgame"] => reset(&mut threads, &tt),

            ["quit"] => break,

            // Non-UCI commands
            ["eval"] => evaluate(&threads.main_thread().board),
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
    println!("option name Hash type spin default {DEFAULT_TT_SIZE} min 1 max 262144");
    println!("option name Threads type spin default 1 min 1 max 256");
    println!("option name Clear Hash type button");

    #[cfg(feature = "spsa")]
    crate::parameters::print_options();

    println!("uciok");
}

fn reset(threads: &mut ThreadPool, tt: &TranspositionTable) {
    threads.clear();
    tt.clear(threads.len());
}

fn go(threads: &mut ThreadPool, tokens: &[&str]) {
    let stm = threads.main_thread().board.side_to_move();
    let limits = parse_limits(stm, tokens);
    threads.main_thread().time_manager = TimeManager::new(limits);
    threads.main_thread().set_stop(false);

    std::thread::scope(|scope| {
        let mut handlers = Vec::new();

        for (id, td) in threads.iter_mut().enumerate() {
            let handler = scope.spawn(move || {
                search::start(td, id != 0);
                td.set_stop(true);

                if id == 0 {
                    println!("bestmove {}", td.pv.best_move());
                }
            });

            handlers.push(handler);
        }

        for handler in handlers {
            handler.join().unwrap();
        }
    });
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
    }
}

fn make_uci_move(board: &mut Board, uci_move: &str) {
    let moves = board.generate_all_moves();
    if let Some(&mv) = moves.iter().find(|mv| mv.to_string() == uci_move) {
        board.make_move::<true, true>(mv);
    }
}

fn set_option(threads: &mut ThreadPool, tt: &TranspositionTable, tokens: &[&str]) {
    match tokens {
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

fn evaluate(board: &Board) {
    let eval = match board.side_to_move() {
        Color::White => board.evaluate(),
        Color::Black => -board.evaluate(),
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
