use crate::cache::{Cache, DEFAULT_CACHE_SIZE, MAX_CACHE_SIZE, MIN_CACHE_SIZE};
use crate::search::Searcher;
use crate::{board::Board, timeman::Limits, tools, types::Color};

pub fn message_loop() {
    let mut board = Board::starting_position();
    let mut cache = Cache::default();

    loop {
        let command = read_stdin();
        let tokens = command.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["uci"] => uci(),
            ["isready"] => println!("readyok"),

            ["go", tokens @ ..] => go(&board, &mut cache, tokens),
            ["position", tokens @ ..] => position(&mut board, tokens),
            ["setoption", tokens @ ..] => set_option(&mut cache, tokens),
            ["ucinewgame"] => reset(&mut board, &mut cache),

            ["quit"] => std::process::exit(0),

            // Non-UCI commands
            ["bench", depth] => tools::bench(depth.parse().unwrap()),
            ["perft", depth] => tools::perft(depth.parse().unwrap(), &mut board),

            _ => eprintln!("Unknown command: '{}'", command.trim_end()),
        };
    }
}

fn uci() {
    println!("id name Reckless {}", env!("CARGO_PKG_VERSION"));
    println!("option name Hash type spin default {DEFAULT_CACHE_SIZE} min {MIN_CACHE_SIZE} max {MAX_CACHE_SIZE}");
    println!("option name ClearHash type button");
    println!("uciok");
}

fn reset(board: &mut Board, cache: &mut Cache) {
    *board = Board::starting_position();
    *cache = Cache::default();
}

fn go(board: &Board, cache: &mut Cache, tokens: &[&str]) {
    let limits = parse_limits(board.turn, tokens);
    let board = board.clone();
    Searcher::new(board, limits, cache).iterative_deepening();
}

fn position(board: &mut Board, mut tokens: &[&str]) {
    loop {
        match tokens {
            ["startpos", rest @ ..] => {
                *board = Board::starting_position();
                tokens = &rest[0..];
            }
            ["fen", rest @ ..] => {
                *board = Board::new(&rest[0..6].join(" "));
                tokens = &rest[6..];
            }
            ["moves", rest @ ..] => {
                rest.iter().for_each(|uci_move| make_uci_move(board, uci_move));
                break;
            }
            _ => break,
        }
    }
}

fn make_uci_move(board: &mut Board, uci_move: &str) {
    let moves = board.generate_moves();
    if let Some(mv) = moves.iter().find(|mv| mv.to_string() == uci_move) {
        board.make_move(mv).expect("UCI move should be legal");
    }
}

fn set_option(cache: &mut Cache, tokens: &[&str]) {
    match tokens {
        ["name", "Hash", "value", v] => *cache = Cache::new(v.parse().unwrap()),
        ["name", "Clear", "Hash"] => cache.clear(),
        _ => eprintln!("Unknown option: '{}'", tokens.join(" ").trim_end()),
    }
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
                "depth" => return Limits::FixedDepth(value as i32),
                "movetime" => return Limits::FixedTime(value),

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
        Limits::Infinite
    } else {
        Limits::Tournament(main, inc, moves)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_time_control {
        ($($name:ident: $input:expr, $expected:expr,)*) => {$(
            #[test]
            fn $name() {
                let tokens = $input.split_whitespace().collect::<Vec<_>>();
                assert_eq!(parse_limits(Color::White, &tokens), $expected);
            }
        )*};
    }

    assert_time_control!(
        tc_infinite: "infinite", Limits::Infinite,
        tc_fixed_time: "movetime 5000", Limits::FixedTime(5000),
        tc_fixed_depth: "depth 10", Limits::FixedDepth(10),
        tc_increment: "winc 750 binc 900", Limits::Tournament(0, 750, None),
        tc_tournament: "wtime 750 winc 900 movestogo 12", Limits::Tournament(750, 900, Some(12)),
        tc_invalid: "invalid", Limits::Infinite,
    );
}
