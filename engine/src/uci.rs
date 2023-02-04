//! The Universal Chess Interface (UCI) is an open communication protocol that enables
//! chess engines to communicate with other programs including Graphical User Interfaces.
//!
//! See [Chess Programming Wiki article](https://www.chessprogramming.org/UCI)
//! for more information.
use crate::Engine;

pub fn execute_command(engine: &mut Engine, input: &str) {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.is_empty() {
        return;
    }

    match tokens[0] {
        "uci" => uci_command(),
        "isready" => is_ready_command(),
        "ucinewgame" => uci_new_game_command(engine),
        "position" => position_command(engine, &tokens),
        "go" => go_command(engine, &tokens),

        // Custom CLI commands not included in the UCI protocol
        "perft" => perft_command(engine, &tokens),
        "eval" => engine.eval(),

        _ => println!("Unknown command '{}'", tokens[0]),
    };
}

/// Use UCI (universal chess interface).
///
/// This command will be sent once as the first command after the program is loaded.
///
/// Upon receiving the uci command, the engine should identify itself with the `id`
/// command and send `option` commands to tell the GUI what engine settings
/// the engine supports, if any.
fn uci_command() {
    println!("id name Reckless");
    println!("uciok");
}

/// Synchronize the engine with the GUI.
///
/// When the GUI has sent a command or commands that may take some time to complete,
/// that command can be used to wait for the engine to be ready again, or to ping
/// the engine to see if it is alive.
///
/// This command should always be answered with `readyok`.
fn is_ready_command() {
    println!("readyok");
}

/// Prepare the engine for a new game.
///
/// When the next search (started with `position` and `go`) will be from another game.
/// This could be a new game the engine should play, or a new game it should analyze.
fn uci_new_game_command(engine: &mut Engine) {
    engine.set_position(Engine::START_FEN)
}

/// Set up the position described in <fenstring> on the internal board and play
/// the moves on the internal chess board.
///
/// If the game was played from the starting position, the string `startpos` should be sent.
///
/// Format: `position [fen <fenstring> | startpos ] moves <move1> ... <movei>`
fn position_command(engine: &mut Engine, tokens: &[&str]) {
    if tokens.len() <= 1 {
        return;
    }

    let token = tokens[1];
    if token == "startpos" {
        engine.set_position(Engine::START_FEN);
    } else if token == "fen" && tokens.len() >= 8 {
        engine.set_position(&tokens[2..8].join(" "));
    }

    if let Some(index) = tokens.iter().position(|&t| t == "moves") {
        for token in &tokens[(index + 1)..] {
            engine.play_uci_move(token);
        }
    }
}

/// Start calculating on the current position set up with the `position` command.
///
/// This command can be followed by a number of arguments, all sent on the same line.
/// If a argument is not sent, its value should be interpreted so that it does not
/// influence the search.
///
/// # Arguments
///
/// * `depth <x>` - Search `x` plies only.
fn go_command(engine: &mut Engine, tokens: &[&str]) {
    if let Some(depth) = parse_token(tokens, "depth") {
        engine.search(depth);
    }
}

/// Run a performance test with the specified depth on the current position set up
/// with the `position` command.
fn perft_command(engine: &mut Engine, tokens: &[&str]) {
    if let Some(depth) = parse_token(tokens, "perft") {
        engine.perft(depth);
    }
}

fn parse_token<T: std::str::FromStr>(tokens: &[&str], token: &str) -> Option<T> {
    let index = tokens.iter().position(|&t| t == token)?;
    let token = tokens.get(index + 1)?;
    token.parse::<T>().ok()
}
