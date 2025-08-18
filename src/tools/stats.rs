use std::{fs::File, io::BufReader};

use crate::tools::BinpackReader;

pub fn stats(inputs: &[String]) {
    const MIN_PLY: usize = 28;
    const MAX_PLY: usize = 160;
    const MAX_SCORE: i16 = 2800;

    let mut total_games = 0;
    let mut total_positions = 0;
    let mut filtered_positions = 0;

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
                    filtered_positions += 1;
                }

                total_positions += 1;
                board.make_move(mv);
            }

            total_games += 1;
        }
    }

    println!("Total games: {total_games}");
    println!("Total positions: {total_positions}");
    println!(
        "Filtered positions: {filtered_positions} ({:.2}%)",
        filtered_positions as f64 / total_positions as f64 * 100.0
    );
}
