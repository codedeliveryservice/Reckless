use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Write},
    path::Path,
};

use crate::{
    board::Board,
    tools::BinpackReader,
    types::{Color, Move},
};

#[derive(Default)]
struct Statistics {
    /// Win/Draw/Loss (0 = black won, 1 = draw, 2 = white won)
    wdl: [usize; 3],
    /// Distribution of game lengths in plies
    lengths: HashMap<u16, usize>,
    /// Distribution of evaluation scores in internal units
    scores: HashMap<i16, usize>,
    /// Distribution of opening evaluation scores (the first position in each game)
    opening_scores: HashMap<i16, usize>,
    /// Distribution of opening ply counts (how many plies until the first move is played)
    opening_ply: HashMap<u16, usize>,
    /// Distribution of piece counts in positions
    pieces: HashMap<u8, usize>,
    /// King positions frequency by square index (mirrored for black)
    king_positions: HashMap<u8, usize>,
}

/// Basic filtering function for positions
fn filter(position: &Board, mv: Move, score: i16) -> bool {
    score.abs() < 4096 && !mv.is_capture() && !mv.is_promotion() && !position.in_check()
}

pub fn stats(inputs: &[String]) {
    for input in inputs {
        let mut statistics = Statistics::default();

        let file = File::open(input).unwrap();
        let mut reader = BinpackReader::new(BufReader::new(file));

        println!("Processing: {input}...");

        while let Some((position, entries)) = reader.next() {
            let mut board = position.to_board();

            for (index, &(mv, score)) in entries.iter().enumerate() {
                let ply = board.fullmove_number() * 2 + index;

                if !filter(&board, mv, score) {
                    board.make_move(mv);
                    continue;
                }

                if index == 0 {
                    *statistics.opening_scores.entry(score).or_insert(0) += 1;
                    *statistics.opening_ply.entry(ply as u16).or_insert(0) += 1;
                }

                *statistics.scores.entry(score).or_insert(0) += 1;
                *statistics.pieces.entry(board.occupancies().len() as u8).or_insert(0) += 1;

                let king_square = match board.side_to_move() {
                    Color::White => board.king_square(Color::White),
                    Color::Black => board.king_square(Color::Black) ^ 56,
                };
                *statistics.king_positions.entry(king_square as u8).or_insert(0) += 1;

                board.make_move(mv);
            }

            *statistics.lengths.entry(entries.len() as u16).or_insert(0) += 1;

            statistics.wdl[position.result as usize] += 1;
        }

        export_statistics(input, statistics);
    }
}

fn export_statistics(input: &String, statistics: Statistics) {
    let path = Path::new(input);
    let base = path.file_stem().and_then(|s| s.to_str()).unwrap().to_string();

    let cwd = std::env::current_dir().unwrap();
    let make_path = move |suffix: &str| cwd.join(format!("{base}{suffix}")).to_string_lossy().into_owned();

    {
        // (wdl -> occurrences)
        let mut f = File::create(make_path(".wdl")).unwrap();
        writeln!(f, "outcome,count").unwrap();
        for (i, &c) in statistics.wdl.iter().enumerate() {
            writeln!(f, "{i},{c}").unwrap();
        }
    }

    {
        // (ply count -> occurrences)
        let mut f = File::create(make_path(".lengths")).unwrap();
        writeln!(f, "ply_count,count").unwrap();
        let mut items: Vec<_> = statistics.lengths.iter().collect();
        items.sort_by_key(|(k, _)| *k);
        for (k, v) in items {
            writeln!(f, "{k},{v}").unwrap();
        }
    }

    {
        // (score -> occurrences)
        let mut f = File::create(make_path(".scores")).unwrap();
        writeln!(f, "score,count").unwrap();
        let mut items: Vec<_> = statistics.scores.iter().collect();
        items.sort_by_key(|(k, _)| *k);
        for (k, v) in items {
            writeln!(f, "{k},{v}").unwrap();
        }
    }

    {
        // (opening_scores -> occurrences)
        let mut f = File::create(make_path(".opening_scores")).unwrap();
        writeln!(f, "score,count").unwrap();
        let mut items: Vec<_> = statistics.opening_scores.iter().collect();
        items.sort_by_key(|(k, _)| *k);
        for (k, v) in items {
            writeln!(f, "{k},{v}").unwrap();
        }
    }

    {
        // (opening_ply -> occurrences)
        let mut f = File::create(make_path(".opening_ply")).unwrap();
        writeln!(f, "ply,count").unwrap();
        let mut items: Vec<_> = statistics.opening_ply.iter().collect();
        items.sort_by_key(|(k, _)| *k);
        for (k, v) in items {
            writeln!(f, "{k},{v}").unwrap();
        }
    }

    {
        // (piece_count -> occurrences)
        let mut f = File::create(make_path(".pieces")).unwrap();
        writeln!(f, "piece_count,count").unwrap();
        let mut items: Vec<_> = statistics.pieces.iter().collect();
        items.sort_by_key(|(k, _)| *k);
        for (k, v) in items {
            writeln!(f, "{k},{v}").unwrap();
        }
    }

    {
        // (king_square_index -> occurrences)
        let mut f = File::create(make_path(".king_positions")).unwrap();
        writeln!(f, "square_index,count").unwrap();
        let mut items: Vec<_> = statistics.king_positions.iter().collect();
        items.sort_by_key(|(k, _)| *k);
        for (k, v) in items {
            writeln!(f, "{k},{v}").unwrap();
        }
    }
}
