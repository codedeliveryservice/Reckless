use crate::engine::Engine;

use super::UciCommand;

pub struct Parser {
    tokens: Vec<String>,
}

impl Parser {
    /// Creates a new `Parser`.
    pub fn new(str: String) -> Self {
        Self {
            tokens: str.split_whitespace().map(|t| t.to_string()).collect(),
        }
    }

    /// Returns a statically typed `UciCommand` parsed from the `String`.
    ///
    /// # Errors
    ///
    /// This function will return an error if the command could not be parsed.
    pub fn parse_command(&self) -> Result<UciCommand, ()> {
        if self.tokens.is_empty() {
            return Err(());
        }

        match self.tokens[0].as_str() {
            "uci" => Ok(UciCommand::Info),
            "isready" => Ok(UciCommand::IsReady),
            "ucinewgame" => Ok(UciCommand::NewGame),

            "stop" => Ok(UciCommand::Stop),
            "quit" => Ok(UciCommand::Quit),

            "eval" => Ok(UciCommand::Eval),

            "position" if self.tokens.len() >= 2 => {
                let fen = match &*self.tokens[1] {
                    "startpos" => Engine::START_FEN.to_string(),
                    "fen" if self.tokens.len() >= 8 => self.tokens[2..8].join(" "),
                    _ => return Err(()),
                };

                let moves = match self.tokens.iter().position(|t| t == "moves") {
                    Some(index) => self.tokens[(index + 1)..].to_vec(),
                    None => vec![],
                };

                Ok(UciCommand::Position { fen, moves })
            }

            "go" => Ok(UciCommand::Search {
                depth: self.parse_token("depth", 6),
            }),

            "perft" => Ok(UciCommand::Perft {
                depth: self.parse_token("depth", 5),
            }),

            _ => Err(()),
        }
    }

    /// Returns the token value if successfully parsed or the specified `default` value.
    fn parse_token<T: std::str::FromStr>(&self, token: &str, default: T) -> T {
        // TODO: Add min and max limits for the value
        match self.try_parse_token(token) {
            Some(value) => value,
            None => default,
        }
    }

    fn try_parse_token<T: std::str::FromStr>(&self, token: &str) -> Option<T> {
        let index = self.tokens.iter().position(|t| t == token)?;
        let token = self.tokens.get(index + 1)?;
        token.parse::<T>().ok()
    }
}
