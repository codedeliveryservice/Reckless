use crate::cache::{DEFAULT_CACHE_SIZE, MAX_CACHE_SIZE, MIN_CACHE_SIZE};
use crate::types::Color;
use crate::{board::Board, engine::Engine, timeman::Limits};

pub fn execute(engine: &mut Engine, command: String) {
    let tokens = command.split_whitespace().collect::<Vec<_>>();

    match tokens.as_slice() {
        ["uci"] => uci(),
        ["isready"] => println!("readyok"),

        ["stop"] => engine.stop(),
        ["ucinewgame"] => engine.reset(),

        ["setoption", tokens @ ..] => set_option(engine, tokens),
        ["position", tokens @ ..] => position(engine, tokens),
        ["go", tokens @ ..] => go(engine, tokens),

        ["quit"] => std::process::exit(0),

        // Non-UCI commands
        ["perft", depth] => perft(engine, depth),

        _ => eprintln!("Unknown command: '{}'", command.trim_end()),
    }
}

fn uci() {
    println!("id name Reckless {}", env!("CARGO_PKG_VERSION"));
    println!("option name Hash type spin default {DEFAULT_CACHE_SIZE} min {MIN_CACHE_SIZE} max {MAX_CACHE_SIZE}");
    println!("option name ClearHash type button");
    println!("uciok");
}

fn set_option(engine: &mut Engine, tokens: &[&str]) {
    if let ["name", name, "value", value] = tokens {
        match *name {
            "Hash" => engine.set_cache_size(value.parse().expect("Cache size should be a number")),
            "ClearHash" => engine.clear_cache(),
            _ => eprintln!("Unknown option: '{name}'"),
        }
    }
}

fn position(engine: &mut Engine, mut tokens: &[&str]) {
    loop {
        match tokens {
            ["startpos", rest @ ..] => {
                engine.board = Board::starting_position();
                tokens = &rest[0..];
            }
            ["fen", rest @ ..] => {
                engine.board = Board::new(&rest[0..6].join(" "));
                tokens = &rest[6..];
            }
            ["moves", rest @ ..] => {
                for uci_move in rest {
                    engine.make_uci_move(uci_move);
                }
                break;
            }
            _ => break,
        }
    }
}

fn perft(engine: &mut Engine, depth: &str) {
    let depth = depth.parse().expect("Depth should be a number");
    engine.perft(depth);
}

fn go(engine: &mut Engine, tokens: &[&str]) {
    let time_control = parse_limits(engine.board.turn, tokens);
    engine.search(time_control);
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
                "depth" => return Limits::FixedDepth(value as usize),
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
        return Limits::Infinite;
    }

    Limits::Tournament(main, inc, moves)
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
