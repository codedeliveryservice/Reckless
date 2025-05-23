use std::sync::atomic::{AtomicBool, AtomicU64};

use crate::{
    board::Board,
    search::{self, Report},
    thread::ThreadData,
    time::{Limits, TimeManager},
    transposition::TranspositionTable,
    types::Move,
};

const RANDOM_PLIES: &[usize] = &[8, 9, 10];
const VALIDATION_THRESHOLD: i32 = 400;
const VALIDATION_LIMITS: Limits = Limits::Depth(10);

pub fn genfens() {
    let args = std::env::args().nth(1).unwrap();
    let args = args.split_whitespace().collect::<Vec<_>>();

    let count = args[1].parse::<u64>().unwrap();
    let seed = args[3].parse::<u64>().unwrap();

    let tt = TranspositionTable::default();
    let stop = AtomicBool::new(false);
    let counter = AtomicU64::new(0);
    let tb_hits = AtomicU64::new(0);

    let mut td = ThreadData::new(&tt, &stop, &counter, &tb_hits);
    let mut random = Random { seed };
    let mut generated = 0;

    while generated < count {
        td.board = generate_random_opening(&mut random);

        let score = validation_score(&mut td);
        if score.abs() >= VALIDATION_THRESHOLD {
            continue;
        }

        println!("info string genfens {}", td.board.to_fen());
        generated += 1;
    }
}

fn generate_random_opening(random: &mut Random) -> Board {
    let mut board = Board::starting_position();

    let plies = RANDOM_PLIES[random.next() % RANDOM_PLIES.len()];
    for _ in 0..plies {
        let moves = generate_legal_moves(&mut board);
        if moves.is_empty() {
            return generate_random_opening(random);
        }

        let index = random.next() % moves.len();
        board.make_move(moves[index]);
        board.advance_fullmove_counter();
    }

    if generate_legal_moves(&mut board).is_empty() {
        return generate_random_opening(random);
    }
    board
}

fn generate_legal_moves(board: &mut Board) -> Vec<Move> {
    board.generate_all_moves().iter().filter(|&v| board.is_legal(v.mv)).map(|v| v.mv).collect()
}

fn validation_score(td: &mut ThreadData) -> i32 {
    td.time_manager = TimeManager::new(VALIDATION_LIMITS, 0, 0);
    search::start(td, Report::None);
    td.best_score
}

struct Random {
    pub seed: u64,
}

impl Random {
    pub fn next(&mut self) -> usize {
        // https://en.wikipedia.org/wiki/Linear_congruential_generator
        self.seed = self.seed.wrapping_mul(0x5851F42D4C957F2D).wrapping_add(1);
        self.seed as usize
    }
}
