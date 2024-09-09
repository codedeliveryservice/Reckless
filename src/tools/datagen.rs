use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{
    board::Board,
    search::{self, Options, SearchResult},
    tables::{History, TranspositionTable},
    time::Limits,
    tools::datagen::random::Random,
    types::{Color, Move},
};

mod position;
mod random;

const VALIDATION_OPTIONS: Options = Options { silent: true, threads: 1, limits: VALIDATION_LIMITS };
const GENERATION_OPTIONS: Options = Options { silent: true, threads: 1, limits: GENERATION_LIMITS };

const REPORT_INTERVAL: Duration = Duration::from_secs(60);
const BUFFER_SIZE: usize = 128 * 1024;

const RANDOM_PLIES: usize = 4;
const AVERAGE_BOOK_PLY: usize = 8;

const VALIDATION_THRESHOLD: i32 = 400;
const GENERATION_THRESHOLD: i32 = 2400;

const DRAW_SCORE: i32 = 20;
const DRAW_PLY_COUNT: i32 = 12;
const DRAW_PLY_NUMBER: usize = 80;

const WRITE_MIN_PLY: usize = 16;
const WRITE_MAX_PLY: usize = 400;

const VALIDATION_LIMITS: Limits = Limits::FixedDepth(10);
const GENERATION_LIMITS: Limits = Limits::FixedNodes(7500);

static STOP_FLAG: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicUsize = AtomicUsize::new(0);
static GAMES: AtomicUsize = AtomicUsize::new(0);

/// Starts the data generation process.
pub fn datagen<P: AsRef<Path>>(output: P, book: P, threads: usize) {
    let reader = BufReader::new(File::open(&book).unwrap());
    let lines = BufReader::new(reader).lines().map(Result::unwrap).collect::<Vec<_>>();

    fs::create_dir_all(&output).unwrap();

    let seed = Random::new().seed as u32;

    println!("Output path          | {}", output.as_ref().display());
    println!("File seed            | {seed:08x}");
    println!("Opening book         | {}", book.as_ref().display());
    println!("Opening positions    | {}", lines.len());
    println!("Threads              | {threads}");
    println!("Random plies         | {RANDOM_PLIES}");
    println!("Validation limits    | {VALIDATION_LIMITS:?}");
    println!("Generation limits    | {GENERATION_LIMITS:?}");
    println!("Validation threshold | {VALIDATION_THRESHOLD}");
    println!("Draw adjudication    | {DRAW_SCORE} for {DRAW_PLY_COUNT} plies, from ply {DRAW_PLY_NUMBER}");
    println!("Write minimum ply    | {WRITE_MIN_PLY}");
    println!("Write maximum ply    | {WRITE_MAX_PLY}");
    println!();
    println!("Press [ENTER] to stop the data generation.");
    println!("Generating data...");
    println!();

    thread::spawn(|| {
        let now = Instant::now();
        loop {
            thread::sleep(REPORT_INTERVAL);

            let positions = COUNT.load(Ordering::Relaxed);
            let games = GAMES.load(Ordering::Relaxed);

            println!(
                "{positions:>8.0} positions | {games:>5} games ({:3.0} pos/game) ({:4.0} pos/s) [{:.1} min]",
                positions as f64 / games as f64,
                positions as f64 / now.elapsed().as_secs_f64(),
                now.elapsed().as_secs_f64() / 60.0,
            );
        }
    });

    thread::scope(|scope| {
        for id in 0..threads {
            let path = output.as_ref().join(format!("{seed:08x}_{id}.bin"));
            let buf = BufWriter::with_capacity(BUFFER_SIZE, File::create(path).unwrap());
            scope.spawn(|| generate_data(buf, &lines));
        }

        std::io::stdin().read_line(&mut String::new()).unwrap();
        STOP_FLAG.store(true, Ordering::Relaxed);
        println!("Stopping data generation...");
    });

    println!("Total positions: {}", COUNT.load(Ordering::Relaxed));
    std::process::exit(0);
}

/// Generates training data for the neural network.
fn generate_data(mut buf: BufWriter<File>, book: &[String]) {
    let mut random = Random::new();

    while !STOP_FLAG.load(Ordering::Relaxed) {
        let mut board = generate_random_opening(&mut random, book);
        let score = validation_score(&mut board);

        if score.abs() >= VALIDATION_THRESHOLD {
            continue;
        }

        let (entries, wdl) = play_game(board.clone());
        let mut count = 0;

        for (index, entry) in entries.iter().enumerate() {
            let ply = AVERAGE_BOOK_PLY + RANDOM_PLIES + index;

            if (WRITE_MIN_PLY..=WRITE_MAX_PLY).contains(&ply)
                && !board.is_in_check()
                && !entry.best_move.is_capture()
                && !entry.best_move.is_promotion()
            {
                let position = position::Position::parse(&board, entry.score, wdl);
                buf.write_all(position.as_bytes()).unwrap();
                count += 1;
            }

            assert!(board.make_move::<false, true>(entry.best_move));
        }

        COUNT.fetch_add(count, Ordering::Relaxed);
        GAMES.fetch_add(1, Ordering::Relaxed);
    }
}

/// Plays a game and returns the search results and the WDL result.
fn play_game(mut board: Board) -> (Vec<SearchResult>, f32) {
    let tt = TranspositionTable::default();
    let mut history = History::default();
    let mut entries = Vec::new();
    let mut draw_counter = 0;

    loop {
        let entry = search::start(GENERATION_OPTIONS, &mut board, &mut history, &tt);
        let SearchResult { best_move, score, .. } = entry;

        draw_counter = if score.abs() <= DRAW_SCORE { draw_counter + 1 } else { 0 };

        // Resignation
        if score.abs() >= GENERATION_THRESHOLD {
            return (entries, winner(&board, score));
        }

        // Draw adjudication
        if draw_counter >= DRAW_PLY_COUNT && entries.len() >= DRAW_PLY_NUMBER {
            return (entries, 0.5);
        }

        entries.push(entry);
        assert!(board.make_move::<true, true>(best_move));

        // Draw by repetition, 50-move rule or insufficient material
        if board.is_draw() || board.draw_by_insufficient_material() {
            return (entries, 0.5);
        }

        // Stalemate
        if generate_legal_moves(&mut board).is_empty() {
            assert!(!board.is_in_check(), "Stalemate in check");
            return (entries, 0.5);
        }
    }
}

fn winner(board: &Board, score: i32) -> f32 {
    match board.side_to_move() {
        Color::White if score > 0 => 1.0,
        Color::Black if score < 0 => 1.0,
        Color::White => 0.0,
        Color::Black => 0.0,
    }
}

/// Generates a random opening position.
fn generate_random_opening(random: &mut Random, book: &[String]) -> Board {
    let index = random.next() % book.len();
    let mut board = Board::new(&book[index]).unwrap();

    for _ in 0..RANDOM_PLIES {
        let moves = generate_legal_moves(&mut board);
        if moves.is_empty() {
            return generate_random_opening(random, book);
        }

        let index = random.next() % moves.len();
        assert!(board.make_move::<true, true>(moves[index]));
    }

    if generate_legal_moves(&mut board).is_empty() {
        return generate_random_opening(random, book);
    }
    board
}

/// Returns the score of the position after performing a validation search.
fn validation_score(board: &mut Board) -> i32 {
    let tt = TranspositionTable::default();
    let mut history = History::default();
    search::start(VALIDATION_OPTIONS, board, &mut history, &tt).score
}

/// Generates all legal moves for the given board.
fn generate_legal_moves(board: &mut Board) -> Vec<Move> {
    let mut legals = Vec::new();
    for &mv in board.generate_all_moves().iter() {
        if !board.make_move::<false, true>(mv) {
            board.undo_move::<false>();
            continue;
        }

        legals.push(mv);
        board.undo_move::<false>();
    }
    legals
}
