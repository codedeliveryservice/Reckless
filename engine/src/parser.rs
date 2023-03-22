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
    if let Some(token) = tokens.get(2) {
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
        time_control: parse_time_control(&tokens[1..], turn)?,
    })
}

fn parse_time_control(tokens: &[&str], turn: Color) -> Result<TimeControl, ()> {
    let mut wtime = None;
    let mut winc = None;
    let mut btime = None;
    let mut binc = None;

    let mut moves_left = None;

    for chunk in tokens.chunks(2) {
        let (token, value) = (chunk[0], chunk.get(1).copied());

        match token {
            "infinite" => return Ok(TimeControl::Infinite),
            "depth" => return Ok(TimeControl::FixedDepth(parse(value)?)),
            "movetime" => return Ok(TimeControl::FixedTime(parse(value)?)),

            "wtime" => wtime = Some(parse(value)?),
            "btime" => btime = Some(parse(value)?),

            "winc" => winc = Some(parse(value)?),
            "binc" => binc = Some(parse(value)?),

            "movestogo" => moves_left = Some(parse(value)?),

            _ => return Err(()),
        }
    }

    let (main, inc) = match turn {
        Color::White => (wtime, winc),
        Color::Black => (btime, binc),
    };

    if main.is_none() && inc.is_none() {
        return Ok(TimeControl::Infinite);
    }

    Ok(match moves_left {
        Some(moves) => TimeControl::Tournament(main.unwrap_or(0), inc.unwrap_or(0), moves),
        None => TimeControl::Incremental(main.unwrap_or(0), inc.unwrap_or(0)),
    })
}

fn parse<T: std::str::FromStr>(value: Option<&str>) -> Result<T, ()> {
    match value {
        Some(value) => value.parse().map_err(|_| ()),
        None => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_tc {
        ($($name:ident: ($tokens:tt, $color:expr, $expected:expr),)*) => {$(
            #[test]
            fn $name() {
                let actual = parse_time_control(&$tokens, $color);
                assert_eq!(actual, $expected)
            }
        )*};
    }

    assert_tc! {
        go_empty: ([], Color::White, Ok(TimeControl::Infinite)),
        go_infinite: (["infinite"], Color::White, Ok(TimeControl::Infinite)),
        go_depth: (["depth", "10"], Color::White, Ok(TimeControl::FixedDepth(10))),
        go_move_time: (["movetime", "5000"], Color::White, Ok(TimeControl::FixedTime(5000))),
        go_wtime_btime: (["wtime", "2000", "btime", "1000"], Color::Black, Ok(TimeControl::Incremental(1000, 0))),
        go_winc_binc: (["winc", "500", "binc", "1000"], Color::White, Ok(TimeControl::Incremental(0, 500))),
        go_full: (["wtime", "2500", "btime", "2100", "winc", "500", "binc", "100", "movestogo", "12"], Color::Black, Ok(TimeControl::Tournament(2100, 100, 12))),
        go_invalid: (["invalid"], Color::White, Err(())),
    }
}
