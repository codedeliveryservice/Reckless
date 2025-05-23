use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use std::collections::HashSet;

use rand::{Rng, SeedableRng, rngs::SmallRng};

const TOTAL_LINES: usize = 56_574_661;

pub fn genfens() {
    let args = std::env::args().nth(1).unwrap();
    let args = args.split_whitespace().collect::<Vec<_>>();

    let count = args[1].parse::<usize>().unwrap();
    let seed = args[3].parse::<u64>().unwrap();
    let book = args[5].to_string();

    let mut rng = SmallRng::seed_from_u64(seed);

    let mut targets = HashSet::with_capacity(count);
    while targets.len() < count {
        targets.insert(rng.random_range(0..TOTAL_LINES));
    }

    let mut targets = targets.into_iter().collect::<Vec<_>>();
    targets.sort_unstable();

    let reader = BufReader::new(File::open(&book).unwrap());
    let mut targets = targets.into_iter().peekable();

    for (index, line) in reader.lines().enumerate() {
        let line = line.unwrap();

        while let Some(&target) = targets.peek() {
            if target > index {
                break;
            }

            let fen = line.split_whitespace().take(6).collect::<Vec<_>>().join(" ");
            println!("info string genfens {fen}");
            targets.next();
        }

        if targets.peek().is_none() {
            break;
        }
    }
}
