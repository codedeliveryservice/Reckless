use game::{Board, Color};
use search::{TimeControl, DEFAULT_CACHE_SIZE, MAX_CACHE_SIZE, MIN_CACHE_SIZE};

use crate::engine::Engine;

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
        ["eval"] => engine.evaluate(),

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
        match name {
            &"Hash" => engine.set_cache_size(value.parse().expect("Cache size should be a number")),
            &"ClearHash" => engine.clear_cache(),
            _ => eprintln!("Unknown option: '{}'", name),
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
    let time_control = parse_time_control(engine.board.turn, tokens);
    engine.search(time_control);
}

fn parse_time_control(color: Color, tokens: &[&str]) -> TimeControl {
    if let ["infinite"] = tokens {
        return TimeControl::Infinite;
    }

    let mut main = 0;
    let mut inc = 0;
    let mut moves = None;

    for chunk in tokens.chunks(2) {
        if let [name, value] = *chunk {
            let value = value.parse().expect("Time control should be a number");

            match name {
                "depth" => return TimeControl::FixedDepth(value as usize),
                "movetime" => return TimeControl::FixedTime(value),

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
        return TimeControl::Infinite;
    }

    match moves {
        Some(moves) => TimeControl::Tournament(main, inc, moves),
        None => TimeControl::Incremental(main, inc),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_time_control {
        ($($name:ident: $input:expr, $color:expr, $expected:expr,)*) => {$(
            #[test]
            fn $name() {
                let tokens = $input.split_whitespace().collect::<Vec<_>>();
                assert_eq!(parse_time_control($color, &tokens), $expected);
            }
        )*};
    }

    assert_time_control!(
        tc_infinite: "infinite", Color::White, TimeControl::Infinite,
        tc_fixed_time: "movetime 5000", Color::White, TimeControl::FixedTime(5000),
        tc_fixed_depth: "depth 10", Color::White, TimeControl::FixedDepth(10),
        tc_time: "wtime 750 btime 900", Color::Black, TimeControl::Incremental(900, 0),
        tc_increment: "winc 750 binc 900", Color::White, TimeControl::Incremental(0, 750),
        tc_tournament: "wtime 750 winc 900 movestogo 12", Color::White, TimeControl::Tournament(750, 900, 12),
        tc_invalid: "invalid", Color::White, TimeControl::Infinite,
    );
}
