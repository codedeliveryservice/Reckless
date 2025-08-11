use std::{fs::File, io::BufReader};

use super::BinpackReader;
use crate::types::Color;

pub fn collect_buckets(inputs: &[String]) {
    const MIN_PLY: usize = 28;
    const MAX_PLY: usize = 160;
    const MAX_SCORE: i16 = 2800;

    let mut buckets = vec![0u64; 64];

    for input in inputs {
        let file = File::open(input).unwrap();
        let mut reader = BinpackReader::new(BufReader::new(file));

        println!("Processing: {input}...");

        while let Some((position, entries)) = reader.next() {
            let mut board = position.to_board();

            for (index, &(mv, score)) in entries.iter().enumerate() {
                let ply = board.fullmove_number() * 2 + index;

                if score.abs() <= MAX_SCORE
                    && (MIN_PLY..=MAX_PLY).contains(&ply)
                    && !mv.is_capture()
                    && !mv.is_promotion()
                    && !board.in_check()
                {
                    let mut king_square = board.king_square(board.side_to_move());

                    if board.side_to_move() == Color::Black {
                        king_square ^= 56;
                    }

                    buckets[king_square as usize] += 1;
                }

                board.make_move(mv);
            }
        }
    }

    println!("{buckets:?}");

    std::process::exit(0);
}
