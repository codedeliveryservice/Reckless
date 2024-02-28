use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{
    board::Board,
    search::{SearchResult, Searcher},
    tables::{HistoryMoves, TranspositionTable},
    timeman::Limits,
    tools::datagen::random::Random,
    types::{Color, Move},
};

mod position;
mod random;

const REPORT_INTERVAL: Duration = Duration::from_secs(30);
const BUFFER_SIZE: usize = 128 * 1024;

const RANDOM_PLIES: usize = 10;
const VALIDATION_THRESHOLD: i32 = 300;
const GENERATION_THRESHOLD: i32 = 1000;

const VALIDATION_LIMITS: Limits = Limits::FixedDepth(10);
const GENERATION_LIMITS: Limits = Limits::FixedNodes(5000);

static STOP_FLAG: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicUsize = AtomicUsize::new(0);

/// Starts the data generation process.
pub fn datagen<P: AsRef<Path>>(output: P, threads: usize) {
    fs::create_dir_all(&output).unwrap();

    let seed = Random::new().seed as u32;

    println!("Output path          | {}", output.as_ref().display());
    println!("File seed            | {seed:08x}");
    println!("Threads              | {threads}");
    println!("Random plies         | {RANDOM_PLIES}");
    println!("Validation limits    | {VALIDATION_LIMITS:?}");
    println!("Generation limits    | {GENERATION_LIMITS:?}");
    println!("Validation threshold | {VALIDATION_THRESHOLD}");
    println!("Generation threshold | {GENERATION_THRESHOLD}");
    println!();
    println!("Press [ENTER] to stop the data generation.");
    println!("Generating data...");
    println!();

    thread::spawn(|| {
        let now = Instant::now();
        loop {
            thread::sleep(REPORT_INTERVAL);
            println!(
                "{:>8.0} positions ({:4.2} pos/s) [{:.1} min]",
                COUNT.load(Ordering::Relaxed),
                COUNT.load(Ordering::Relaxed) as f32 / now.elapsed().as_secs_f32(),
                now.elapsed().as_secs_f32() / 60.0,
            );
        }
    });

    thread::scope(|scope| {
        for id in 0..threads {
            let path = output.as_ref().join(format!("{seed:08x}_{id}.bin"));
            let buf = BufWriter::with_capacity(BUFFER_SIZE, File::create(path).unwrap());
            scope.spawn(move || generate_data(buf));
        }

        std::io::stdin().read_line(&mut String::new()).unwrap();
        STOP_FLAG.store(true, Ordering::Relaxed);
        println!("Stopping data generation...");
    });

    println!("Total positions: {}", COUNT.load(Ordering::Relaxed));
    std::process::exit(0);
}

/// Generates training data for the neural network.
fn generate_data(mut buf: BufWriter<File>) {
    let mut random = Random::new();

    while !STOP_FLAG.load(Ordering::Relaxed) {
        let mut board = generate_random_opening(&mut random);
        let score = validation_score(&mut board);

        if score.abs() >= VALIDATION_THRESHOLD {
            continue;
        }

        let (entries, wdl) = play_game(board.clone());
        let mut count = 0;

        for entry in entries {
            if !board.is_in_check() && !entry.best_move.is_capture() && !entry.best_move.is_promotion() {
                let position = position::Position::parse(&board, entry.score, wdl);
                buf.write_all(position.as_bytes()).unwrap();
                count += 1;
            }

            board.make_move::<false>(entry.best_move).unwrap();
        }

        COUNT.fetch_add(count, Ordering::Relaxed);
    }
}

/// Plays a game and returns the search results and the WDL result.
fn play_game(mut board: Board) -> (Vec<SearchResult>, f32) {
    let mut history = HistoryMoves::default();
    let mut tt = TranspositionTable::default();
    let mut entries = Vec::new();

    loop {
        let mut searcher = Searcher::new(GENERATION_LIMITS, &mut board, &mut history, &mut tt);
        searcher.silent(true);

        let entry = searcher.run();
        let SearchResult { best_move, score } = entry;

        // The score is so high that the game is already decided
        if score.abs() >= GENERATION_THRESHOLD {
            let wdl = match board.side_to_move {
                Color::White if score > 0 => 1.0,
                Color::Black if score < 0 => 1.0,
                Color::White => 0.0,
                Color::Black => 0.0,
            };
            return (entries, wdl);
        }

        entries.push(entry);
        board.make_move::<true>(best_move).unwrap();

        // Draw by repetition or 50-move rule
        if board.is_draw() {
            return (entries, 0.5);
        }

        // Stalemate
        if generate_legal_moves(&mut board).is_empty() {
            assert!(!board.is_in_check(), "Stalemate in check");
            return (entries, 0.5);
        }
    }
}

/// Generates a random opening position.
fn generate_random_opening(random: &mut Random) -> Board {
    let mut board = Board::starting_position();

    for _ in 0..RANDOM_PLIES {
        let moves = generate_legal_moves(&mut board);
        if moves.is_empty() {
            return generate_random_opening(random);
        }

        let index = random.next() % moves.len();
        board.make_move::<true>(moves[index]).unwrap();
    }

    if generate_legal_moves(&mut board).is_empty() {
        return generate_random_opening(random);
    }
    board
}

/// Returns the score of the position after performing a validation search.
fn validation_score(board: &mut Board) -> i32 {
    let mut history = HistoryMoves::default();
    let mut tt = TranspositionTable::default();
    let mut searcher = Searcher::new(VALIDATION_LIMITS, board, &mut history, &mut tt);
    searcher.silent(true);
    searcher.run().score
}

/// Generates all legal moves for the given board.
fn generate_legal_moves(board: &mut Board) -> Vec<Move> {
    let mut legals = Vec::new();
    for mv in board.generate_all_moves().iter() {
        if board.make_move::<false>(mv).is_ok() {
            legals.push(mv);
            board.undo_move::<false>();
        }
    }
    legals
}
