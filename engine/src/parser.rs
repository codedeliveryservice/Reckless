use crate::engine::Engine;

use super::UciCommand;

/// Returns a statically typed `UciCommand` parsed from the `String`.
///
/// # Errors
///
/// This function will return an error if the command could not be parsed.
pub fn parse_command(str: &str) -> Result<UciCommand, ()> {
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
            white_time: try_parse_token(&tokens, "wtime"),
            black_time: try_parse_token(&tokens, "btime"),
            white_inc: try_parse_token(&tokens, "winc"),
            black_inc: try_parse_token(&tokens, "binc"),
            moves: try_parse_token(&tokens, "movestogo"),
            movetime: try_parse_token(&tokens, "movetime"),
            depth: try_parse_token(&tokens, "depth"),
        }),

        "perft" => match try_parse_token::<usize>(&tokens, "perft") {
            Some(depth) => Ok(UciCommand::Perft { depth }),
            None => Err(()),
        },

        _ => Err(()),
    }
}

/// Returns the token value if successfully parsed.
fn try_parse_token<T: std::str::FromStr>(tokens: &[&str], token: &str) -> Option<T> {
    let index = tokens.iter().position(|&t| t == token)?;
    let token = tokens.get(index + 1)?;
    token.parse::<T>().ok()
}
