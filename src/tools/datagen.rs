use std::{
    fs::{self, File},
    io::{BufRead, BufReader, BufWriter},
    path::Path,
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{
    board::Board,
    search::{self, Report, SearchResult},
    thread::ThreadData,
    time::{Limits, TimeManager},
    tools::datagen::random::Random,
    transposition::TranspositionTable,
    types::{Color, Move},
};

#[allow(dead_code)]
pub mod bulletformat;

mod binpack;
mod position;
mod random;

const REPORT_INTERVAL: Duration = Duration::from_secs(60);
const BUFFER_SIZE: usize = 128 * 1024;

const RANDOM_PLIES: usize = 4;
const VALIDATION_THRESHOLD: i32 = 400;

const WIN_SCORE: i32 = 2000;
const WIN_PLY_COUNT: i32 = 8;

const DRAW_SCORE: i32 = 15;
const DRAW_PLY_COUNT: i32 = 12;
const DRAW_PLY_NUMBER: usize = 64;

const VALIDATION_LIMITS: Limits = Limits::Depth(10);
const GENERATION_LIMITS: Limits = Limits::Nodes(5000);

static STOP_FLAG: AtomicBool = AtomicBool::new(false);
static COUNT: AtomicUsize = AtomicUsize::new(0);
static GAMES: AtomicUsize = AtomicUsize::new(0);

pub fn datagen<P: AsRef<Path>>(output: P, book: P, threads: usize) {
    let reader = BufReader::new(File::open(&book).unwrap());
    let lines = BufReader::new(reader).lines().map(Result::unwrap).collect::<Vec<_>>();

    fs::create_dir_all(&output).unwrap();

    println!("Output path          | {}", output.as_ref().display());
    println!("Opening book         | {}", book.as_ref().display());
    println!("Opening positions    | {}", lines.len());
    println!("Threads              | {threads}");
    println!("Random plies         | {RANDOM_PLIES}");
    println!("Validation limits    | {VALIDATION_LIMITS:?}");
    println!("Generation limits    | {GENERATION_LIMITS:?}");
    println!("Validation threshold | score={VALIDATION_THRESHOLD}");
    println!("Win adjudication     | score={WIN_SCORE} plycount={WIN_PLY_COUNT}");
    println!("Draw adjudication    | score={DRAW_SCORE} plycount={DRAW_PLY_COUNT} plynumber={DRAW_PLY_NUMBER}");
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
            let elapsed = now.elapsed().as_secs_f64();

            println!(
                "{games:>8} games ({:4.0} games/s) | {positions:>10} positions ({:4.0} positions/s) | {:.0} min",
                games as f64 / elapsed,
                positions as f64 / elapsed,
                elapsed / 60.0,
            );
        }
    });

    thread::scope(|scope: &thread::Scope<'_, '_>| {
        for _ in 0..threads {
            let path = output.as_ref();
            scope.spawn(|| generate_data(path, &lines));
        }

        std::io::stdin().read_line(&mut String::new()).unwrap();
        STOP_FLAG.store(true, Ordering::Relaxed);
        println!("Stopping data generation...");
    });

    println!("Total positions: {}", COUNT.load(Ordering::Relaxed));
    std::process::exit(0);
}

/// Generates training data for the neural network.
fn generate_data(path: &Path, book: &[String]) {
    let mut random = Random::new();

    let path = path.join(format!("{:016x}.rbinpack", random.seed));
    let buf = BufWriter::with_capacity(BUFFER_SIZE, File::create(path).unwrap());

    let mut writer = binpack::BinpackWriter::new(buf);

    while !STOP_FLAG.load(Ordering::Relaxed) {
        let board = generate_random_opening(&mut random, book);

        let counter = AtomicU64::new(0);
        let tt = TranspositionTable::default();
        let mut td = ThreadData::new(&tt, &STOP_FLAG, &counter);
        td.board = board.clone();

        let score = validation_score(&mut td);
        if score.abs() >= VALIDATION_THRESHOLD {
            continue;
        }

        let (entries, result) = play_game(&mut td);

        writer.write(position::Position::new(&board, result), &entries);

        COUNT.fetch_add(entries.len(), Ordering::Relaxed);
        GAMES.fetch_add(1, Ordering::Relaxed);
    }
}

/// Plays a game and returns the search results and the WDL result.
fn play_game(td: &mut ThreadData) -> (Vec<SearchResult>, u8) {
    td.time_manager = TimeManager::new(GENERATION_LIMITS, 0);

    let mut entries = Vec::new();
    let mut draw_counter = 0;
    let mut win_counter = 0;

    loop {
        let entry = search::start(td, Report::None);
        let SearchResult { best_move, score } = entry;

        draw_counter = if score.abs() <= DRAW_SCORE { draw_counter + 1 } else { 0 };
        win_counter = if score.abs() >= WIN_SCORE { win_counter + 1 } else { 0 };

        // Resignation
        if win_counter >= WIN_PLY_COUNT || score.abs() >= 2 * WIN_SCORE {
            return (entries, winner(&td.board, score));
        }

        // Draw adjudication
        if draw_counter >= DRAW_PLY_COUNT && entries.len() >= DRAW_PLY_NUMBER {
            return (entries, 1);
        }

        entries.push(entry);
        td.board.make_move(best_move);

        // Draw by repetition, 50-move rule or insufficient material
        if td.board.is_draw() || td.board.draw_by_insufficient_material() {
            return (entries, 1);
        }

        // Stalemate
        if generate_legal_moves(&mut td.board).is_empty() {
            return (entries, 1);
        }
    }
}

fn winner(board: &Board, score: i32) -> u8 {
    match board.side_to_move() {
        Color::White if score > 0 => 2,
        Color::Black if score < 0 => 2,
        Color::White => 0,
        Color::Black => 0,
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
        board.make_move(moves[index]);
        board.increment_game_ply();
    }

    if generate_legal_moves(&mut board).is_empty() {
        return generate_random_opening(random, book);
    }
    board
}

/// Returns the score of the position after performing a validation search.
fn validation_score(td: &mut ThreadData) -> i32 {
    td.time_manager = TimeManager::new(VALIDATION_LIMITS, 0);
    search::start(td, Report::None).score
}

fn generate_legal_moves(board: &mut Board) -> Vec<Move> {
    board.generate_all_moves().iter().filter(|&v| board.is_legal(v.mv)).map(|v| v.mv).collect()
}
