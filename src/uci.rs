use crate::{
    board::Board,
    search::Searcher,
    tables::{ContinuationHistory, HistoryMoves, TranspositionTable},
    timeman::Limits,
    tools,
    types::Color,
};

pub fn message_loop() {
    let mut board = Board::starting_position();
    let mut tt = TranspositionTable::default();
    let mut history = HistoryMoves::default();
    let mut fmh = ContinuationHistory::new();

    loop {
        let command = read_stdin();
        let tokens = command.split_whitespace().collect::<Vec<_>>();
        match tokens.as_slice() {
            ["uci"] => uci(),
            ["isready"] => println!("readyok"),

            ["go", tokens @ ..] => go(&mut board, &mut history, &mut fmh, &mut tt, tokens),
            ["position", tokens @ ..] => position(&mut board, tokens),
            ["setoption", tokens @ ..] => set_option(&mut tt, tokens),
            ["ucinewgame"] => reset(&mut board, &mut history, &mut tt),

            ["quit"] => std::process::exit(0),

            // Non-UCI commands
            ["eval"] => evaluate(&board),
            ["bench", depth] => tools::bench::<true>(depth.parse().unwrap()),
            ["perft", depth] => tools::perft(depth.parse().unwrap(), &mut board),

            _ => eprintln!("Unknown command: '{}'", command.trim_end()),
        };
    }
}

fn uci() {
    use crate::tables::DEFAULT_TT_SIZE;

    println!("id name Reckless {}", env!("CARGO_PKG_VERSION"));
    println!("option name Hash type spin default {DEFAULT_TT_SIZE} min 1 max 65536");
    println!("option name Clear Hash type button");
    println!("uciok");
}

fn reset(board: &mut Board, history: &mut HistoryMoves, tt: &mut TranspositionTable) {
    tt.clear();
    *board = Board::starting_position();
    *history = HistoryMoves::default();
}

fn go(board: &mut Board, history: &mut HistoryMoves, fmh: &mut ContinuationHistory, tt: &mut TranspositionTable, tokens: &[&str]) {
    let limits = parse_limits(board.side_to_move, tokens);
    Searcher::new(limits, board, history, fmh, tt).run();
}

fn position(board: &mut Board, mut tokens: &[&str]) {
    while !tokens.is_empty() {
        match tokens {
            ["startpos", rest @ ..] => {
                *board = Board::starting_position();
                tokens = &rest;
            }
            ["fen", rest @ ..] => {
                match Board::new(&rest.join(" ")) {
                    Ok(b) => *board = b,
                    Err(e) => eprintln!("Invalid FEN: {e:?}"),
                }
                tokens = &rest;
            }
            ["moves", rest @ ..] => {
                rest.iter().for_each(|uci_move| make_uci_move(board, uci_move));
                break;
            }
            _ => {
                tokens = &tokens[1..];
                continue;
            }
        }
    }
}

fn make_uci_move(board: &mut Board, uci_move: &str) {
    let moves = board.generate_all_moves();
    if let Some(mv) = moves.iter().find(|mv| mv.to_string() == uci_move) {
        board.make_move::<true>(mv).expect("UCI move should be legal");
    }
}

fn set_option(tt: &mut TranspositionTable, tokens: &[&str]) {
    match tokens {
        ["name", "Clear", "Hash"] => tt.clear(),
        ["name", "Hash", "value", v] => tt.resize(v.parse().unwrap()),
        _ => eprintln!("Unknown option: '{}'", tokens.join(" ").trim_end()),
    }
}

fn evaluate(board: &Board) {
    let eval = match board.side_to_move {
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
                "depth" if value > 0 => return Limits::FixedDepth(value as i32),
                "movetime" if value > 0 => return Limits::FixedTime(value),
                "nodes" if value > 0 => return Limits::FixedNodes(value),

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
        Some(moves) => Limits::Tournament(main, inc, moves),
        None => Limits::Incremental(main, inc),
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
        tc_increment: "winc 750 binc 900", Limits::Incremental(0, 750),
        tc_tournament: "wtime 750 winc 900 movestogo 12", Limits::Tournament(750, 900, 12),
        tc_invalid: "invalid", Limits::Infinite,
    );
}
