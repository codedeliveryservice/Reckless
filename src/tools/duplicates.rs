use std::{fs::File, io::BufReader};

use crate::tools::BinpackReader;

const SNAPSHOT_PLY: usize = 48;

pub fn duplicates(inputs: &[String]) {
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = 0;

    for input in inputs {
        let file = File::open(input).unwrap();
        let mut reader = BinpackReader::new(BufReader::new(file));

        println!("Processing: {input}...");

        while let Some((position, entries)) = reader.next() {
            let mut board = position.to_board();

            for (index, &(mv, _)) in entries.iter().enumerate() {
                let ply = position.fullmove_number as usize * 2 + index;
                if ply >= SNAPSHOT_PLY {
                    break;
                }

                board.make_move(mv);
            }

            board.update_hash_keys();

            if !seen.insert(board.hash()) {
                duplicates += 1;
            }
        }
    }

    println!("Found {duplicates} duplicates ({:.4}%)", (duplicates as f64 / (seen.len() + duplicates) as f64) * 100.0);
    println!("Total unique positions: {}", seen.len());

    std::process::exit(0);
}
