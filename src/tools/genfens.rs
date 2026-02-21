use std::{
    fs::File,
    io::{BufRead, BufReader},
    sync::Arc,
};

use rand::{Rng, SeedableRng, rngs::StdRng};

use crate::{
    board::{Board, NullBoardObserver},
    search::{self, Report},
    thread::{SharedContext, Status, ThreadData},
    time::{Limits, TimeManager},
    types::{Move, is_decisive, normalize_to_cp},
};

const RANDOM_PLIES: &[usize] = &[4, 5];

const VALIDATION_ABS_MIN_CP: i32 = 25;
const VALIDATION_ABS_MAX_CP: i32 = 150;
const VALIDATION_LIMITS: Limits = Limits::Nodes(40_000);

const OPENING_SEARCH_LIMITS: Limits = Limits::Nodes(4_000);
const OPENING_MULTIPV_LINES: usize = 4;

pub fn genfens() {
    let args = std::env::args().nth(1).unwrap();
    let args = args.split_whitespace().collect::<Vec<_>>();

    let count = args[1].parse::<u64>().unwrap();
    let seed = args[3].parse::<u64>().unwrap();
    let book = args[5].to_string();

    let reader = File::open(&book).unwrap();
    let lines = BufReader::new(reader).lines().map(Result::unwrap).collect::<Vec<_>>();

    let shared = Arc::new(SharedContext::default());
    let mut td = ThreadData::new(shared);
    let mut rng = StdRng::seed_from_u64(seed);
    let mut generated = 0;

    while generated < count {
        generate_random_opening(&mut rng, &lines, &mut td);

        let score = normalize_to_cp(validation_score(&mut td), &td.board);
        if score.abs() < VALIDATION_ABS_MIN_CP || score.abs() > VALIDATION_ABS_MAX_CP {
            continue;
        }

        println!("info string genfens {}", td.board.to_fen());
        generated += 1;
    }
}

fn generate_random_opening(rng: &mut StdRng, book: &[String], td: &mut ThreadData) {
    let index = rng.random_range(0..book.len());
    td.board = Board::from_fen(&book[index]).unwrap();

    let plies = RANDOM_PLIES[rng.random_range(0..RANDOM_PLIES.len())];
    for _ in 0..plies {
        let Some(mv) = choose_move(rng, td) else {
            return generate_random_opening(rng, book, td);
        };

        td.board.make_move(mv, &mut NullBoardObserver {});
        td.board.advance_fullmove_counter();
    }

    if generate_legal_moves(&mut td.board).is_empty() {
        return generate_random_opening(rng, book, td);
    }
}

fn generate_legal_moves(board: &mut Board) -> Vec<Move> {
    board.generate_all_moves().iter().filter(|&v| board.is_legal(v.mv)).map(|v| v.mv).collect()
}

fn choose_move(rng: &mut StdRng, td: &mut ThreadData) -> Option<Move> {
    if generate_legal_moves(&mut td.board).is_empty() {
        return None;
    }

    td.time_manager = TimeManager::new(OPENING_SEARCH_LIMITS, 0, 0);

    td.multi_pv = OPENING_MULTIPV_LINES;
    td.shared.nodes.reset();
    td.shared.status.set(Status::RUNNING);

    search::start(td, Report::None, 1);

    let candidates = td.root_moves.iter().filter(|rm| !is_decisive(rm.score)).map(|rm| rm.mv).collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }

    let index = rng.random_range(0..candidates.len());
    Some(candidates[index])
}

fn validation_score(td: &mut ThreadData) -> i32 {
    td.time_manager = TimeManager::new(VALIDATION_LIMITS, 0, 0);

    td.shared.nodes.reset();
    td.shared.status.set(Status::RUNNING);

    search::start(td, Report::None, 1);

    td.root_moves[0].score
}
