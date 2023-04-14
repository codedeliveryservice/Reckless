use game::{Color, STARTING_FEN};
use search::TimeControl;

use crate::commands::UciCommand;

/// Returns a statically typed `UciCommand` parsed from the `String`.
///
/// # Errors
///
/// This function will return an error if the command could not be parsed.
pub fn parse_command(str: &str, turn: Color) -> Result<UciCommand, ()> {
    let tokens: Vec<&str> = str.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(());
    }

    match tokens[0] {
        "uci" => Ok(UciCommand::Info),
        "isready" => Ok(UciCommand::IsReady),
        "ucinewgame" => Ok(UciCommand::NewGame),

        "stop" => Ok(UciCommand::Stop),
        "quit" => Ok(UciCommand::Quit),

        "position" => parse_position_command(tokens),
        "go" => parse_go_command(tokens, turn),

        // Non-UCI commands
        "eval" => Ok(UciCommand::Eval),
        "perft" => parse_perft_command(tokens),

        _ => Err(()),
    }
}

fn parse_perft_command(tokens: Vec<&str>) -> Result<UciCommand, ()> {
    if let Some(token) = tokens.get(1) {
        if let Ok(depth) = token.parse::<usize>() {
            return Ok(UciCommand::Perft { depth });
        }
    }
    Err(())
}

fn parse_position_command(tokens: Vec<&str>) -> Result<UciCommand, ()> {
    if tokens.len() < 2 {
        return Err(());
    }

    let fen = match tokens[1] {
        "startpos" => STARTING_FEN.to_owned(),
        "fen" if tokens.len() >= 8 => tokens[2..8].join(" "),
        _ => return Err(()),
    };

    let moves = match tokens.iter().position(|&t| t == "moves") {
        Some(index) => tokens[(index + 1)..].to_vec(),
        None => vec![],
    };

    Ok(UciCommand::Position { fen, moves })
}

fn parse_go_command(tokens: Vec<&str>, turn: Color) -> Result<UciCommand, ()> {
    Ok(UciCommand::Search {
        time_control: parse_time_control(&tokens[1..], turn),
    })
}

/// Parses a time control command from a list of tokens and returns a `TimeControl` instance.
///
/// If the tokens are invalid, returns `TimeControl::Infinite`.
fn parse_time_control(tokens: &[&str], turn: Color) -> TimeControl {
    try_parse_time_control(tokens, turn).unwrap_or(TimeControl::Infinite)
}

/// Tries to parse the given list of tokens into a `TimeControl` based on the given turn color.
fn try_parse_time_control(tokens: &[&str], turn: Color) -> Result<TimeControl, ()> {
    let mut main = 0;
    let mut inc = 0;
    let mut moves_left = None;

    for chunk in tokens.chunks(2) {
        let (token, value) = (chunk[0], chunk.get(1).copied());

        match token {
            "infinite" => return Ok(TimeControl::Infinite),
            "depth" => return Ok(TimeControl::FixedDepth(parse(value)?)),
            "movetime" => return Ok(TimeControl::FixedTime(parse(value)?)),

            "wtime" if turn == Color::White => main = parse(value)?,
            "btime" if turn == Color::Black => main = parse(value)?,
            "winc" if turn == Color::White => inc = parse(value)?,
            "binc" if turn == Color::Black => inc = parse(value)?,
            "movestogo" => moves_left = Some(parse(value)?),

            _ => continue,
        }
    }

    let is_invalid_time_control = main == 0 && inc == 0;
    if is_invalid_time_control {
        return Err(());
    }

    match moves_left {
        Some(moves) => Ok(TimeControl::Tournament(main, inc, moves)),
        None => Ok(TimeControl::Incremental(main, inc)),
    }
}

/// Parse a string into a type that implements `FromStr`.
fn parse<T: std::str::FromStr>(value: Option<&str>) -> Result<T, ()> {
    value.and_then(|v| v.parse().ok()).ok_or(())
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_tc {
        ($($name:ident: ($tokens:tt, $color:expr, $expected:expr),)*) => {$(
            #[test]
            fn $name() {
                let actual = parse_time_control(&$tokens, $color);
                assert_eq!(actual, $expected);
            }
        )*};
    }

    assert_tc! {
        go_empty: ([], Color::White, TimeControl::Infinite),
        go_infinite: (["infinite"], Color::White, TimeControl::Infinite),
        go_depth: (["depth", "10"], Color::White, TimeControl::FixedDepth(10)),
        go_move_time: (["movetime", "5000"], Color::White, TimeControl::FixedTime(5000)),
        go_wtime_btime: (["wtime", "2000", "btime", "1000"], Color::Black, TimeControl::Incremental(1000, 0)),
        go_winc_binc: (["winc", "500", "binc", "1000"], Color::White, TimeControl::Incremental(0, 500)),
        go_full: (["wtime", "2500", "btime", "2100", "winc", "500", "binc", "100", "movestogo", "12"], Color::Black, TimeControl::Tournament(2100, 100, 12)),
        go_invalid: (["invalid"], Color::White, TimeControl::Infinite),
    }
}
