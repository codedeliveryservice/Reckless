use game::Color;
use search::TimeControl;

use crate::{commands::UciCommand, engine::Engine};

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

        "eval" => Ok(UciCommand::Eval),

        "position" if tokens.len() >= 2 => {
            let fen = match tokens[1] {
                "startpos" => Engine::START_FEN.to_owned(),
                "fen" if tokens.len() >= 8 => tokens[2..8].join(" "),
                _ => return Err(()),
            };

            let moves = match tokens.iter().position(|&t| t == "moves") {
                Some(index) => tokens[(index + 1)..].to_vec(),
                None => vec![],
            };

            Ok(UciCommand::Position { fen, moves })
        }

        "go" => Ok(UciCommand::Search {
            time_control: parse_time_control(&tokens[1..], turn)?,
        }),

        "perft" => match try_parse_token::<usize>(&tokens, "perft") {
            Some(depth) => Ok(UciCommand::Perft { depth }),
            None => Err(()),
        },

        _ => Err(()),
    }
}

fn parse_time_control(tokens: &[&str], turn: Color) -> Result<TimeControl, ()> {
    let mut wtime = None;
    let mut winc = None;
    let mut btime = None;
    let mut binc = None;

    let mut moves_left = None;

    for chunks in tokens.chunks(2) {
        let (token, value) = (chunks[0], chunks[1]);

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

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, ()> {
    value.parse().map_err(|_| ())
}

/// Returns the token value if successfully parsed.
fn try_parse_token<T: std::str::FromStr>(tokens: &[&str], token: &str) -> Option<T> {
    let index = tokens.iter().position(|&t| t == token)?;
    let token = tokens.get(index + 1)?;
    token.parse::<T>().ok()
}
