use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::Arc,
};

use crate::{
    board::Board,
    thread::{SharedContext, ThreadData},
};

pub fn scale(input: &Path) {
    let f = File::open(input).unwrap();

    let shared = Arc::new(SharedContext::default());
    let mut td = ThreadData::new(shared);

    let mut count = 0i128;

    let mut total = 0i128;
    let mut abs_total = 0i128;
    let mut sq_total = 0i128;

    let mut min = i32::MAX;
    let mut max = i32::MIN;

    let lines = BufReader::new(f).lines().collect::<Result<Vec<_>, _>>().unwrap();
    let len = lines.len();

    for (i, line) in lines.into_iter().enumerate() {
        let fen = &line[..line.match_indices(' ').nth(5).map(|(idx, _)| idx).unwrap()];

        td.board = Board::from_fen(fen).unwrap();
        td.nnue.full_refresh(&td.board);

        if td.board.in_check() {
            continue;
        }

        let eval = td.nnue.evaluate(&td.board);

        count += 1;

        total += eval as i128;
        abs_total += (eval.abs()) as i128;
        sq_total += (eval as i128) * (eval as i128);

        min = min.min(eval);
        max = max.max(eval);

        if i % 1024 == 0 {
            let progress = i + 1;
            let percent = progress as f64 * 100.0 / len as f64;
            print!("\rProcessed {progress:>10}/{len} ({percent:5.2}%)");
        }
    }

    println!("\rProcessed {len:>10}/{len} (100.00%)");

    let mean = total as f64 / count as f64;
    let abs_mean = abs_total as f64 / count as f64;
    let mean_squared = mean * mean;
    let variance = (sq_total as f64 / count as f64) - mean_squared;
    let std_dev = variance.sqrt();
    let rms = (sq_total as f64 / count as f64).sqrt();

    println!();
    println!("{:20} {count:>12}", "Count");
    println!("{:20} {mean:>12.4}", "Mean");
    println!("{:20} {abs_mean:>12.4}", "Absolute mean");
    println!("{:20} {rms:>12.4}", "RMS");
    println!("{:20} {std_dev:>12.4}", "Standard deviation");
    println!("{:20} {min:>12.4}", "Min");
    println!("{:20} {max:>12.4}", "Max");

    println!();
    println!("Recommended scaling factor: {}", (388.0 * 1150.30 / abs_mean).round());
}
