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
    types::{Move, normalize_to_cp},
};

const RANDOM_PLIES: &[usize] = &[4, 5];

const VALIDATION_ABS_MIN_CP: i32 = 25;
const VALIDATION_ABS_MAX_CP: i32 = 150;
const VALIDATION_LIMITS: Limits = Limits::Nodes(40_000);

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
        td.board = generate_random_opening(&mut rng, &lines);

        let score = normalize_to_cp(validation_score(&mut td), &td.board);
        if score.abs() < VALIDATION_ABS_MIN_CP || score.abs() > VALIDATION_ABS_MAX_CP {
            continue;
        }

        println!("info string genfens {}", td.board.to_fen());
        generated += 1;
    }
}

fn generate_random_opening(rng: &mut StdRng, book: &[String]) -> Board {
    let index = rng.random_range(0..book.len());
    let mut board = Board::from_fen(&book[index]).unwrap();

    let plies = RANDOM_PLIES[rng.random_range(0..RANDOM_PLIES.len())];
    for _ in 0..plies {
        let moves = generate_legal_moves(&mut board);
        if moves.is_empty() {
            return generate_random_opening(rng, book);
        }

        let index = rng.random_range(0..moves.len());
        board.make_move(moves[index], &mut NullBoardObserver {});
        board.advance_fullmove_counter();
    }

    if generate_legal_moves(&mut board).is_empty() {
        return generate_random_opening(rng, book);
    }
    board
}

fn generate_legal_moves(board: &mut Board) -> Vec<Move> {
    board.generate_all_moves().iter().filter(|&v| board.is_legal(v.mv)).map(|v| v.mv).collect()
}

fn validation_score(td: &mut ThreadData) -> i32 {
    td.time_manager = TimeManager::new(VALIDATION_LIMITS, 0, 0);

    td.shared.nodes.reset();
    td.shared.status.set(Status::RUNNING);

    search::start(td, Report::None, 1);

    td.root_moves[0].score
}
