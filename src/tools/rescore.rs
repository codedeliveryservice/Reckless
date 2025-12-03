use std::sync::Arc;
use std::{fs::File, io::BufReader, io::BufWriter, path::Path};

use crate::board::Board;
use crate::thread::{SharedContext, ThreadData};
use crate::tools::BinpackReader;
use crate::tools::BinpackWriter;
use crate::types::Move;

pub fn rescore(input: String, output: String) {
    assert!(input != output, "Rescoring in-place is not supported");

    println!("Rescoring: {input} -> {output}...");

    let file = File::open(input).unwrap();
    let mut reader = BinpackReader::new(BufReader::new(file));

    let output_path = Path::new(&output);
    let output_file = File::create(output_path).unwrap();
    let mut writer = BinpackWriter::new(BufWriter::new(output_file));

    let shared = Arc::new(SharedContext::default());
    let mut td = ThreadData::new(shared);

    let mut games = 0;
    let mut out_of_bounds = 0;

    let mut original_stats = Stats::default();
    let mut rescored_stats = Stats::default();

    while let Some((position, entries)) = reader.next() {
        let mut rescored_entries = Vec::new();

        td.board = position.to_board();

        for &(mv, score) in entries.iter() {
            td.nnue.full_refresh(&td.board);
            let raw = td.nnue.evaluate(&td.board);
            let clamped = raw.clamp(-16384, 16384) as i16;

            out_of_bounds += (raw != clamped as i32) as usize;

            if filter(&td.board, mv, score) {
                original_stats.update(score);
                rescored_stats.update(clamped);
            }

            rescored_entries.push((mv, clamped));

            td.board.make_move(mv, |_, _, _, _| ());
        }

        writer.write(&position.to_board(), position.result, &rescored_entries);
        games += 1;

        if games % (1024 * 128) == 0 {
            println!("Processed {games} games");
        }
    }

    println!("Games processed: {games}");
    println!("Out of bounds evaluations: {out_of_bounds}");
    println!();

    println!("Original scores statistics:\n{original_stats}");
    println!("Rescored statistics:\n{rescored_stats}");
}

fn filter(board: &Board, mv: Move, score: i16) -> bool {
    score.abs() < 4096 && !mv.is_capture() && !mv.is_promotion() && !board.in_check()
}

struct Stats {
    count: i128,
    total: i128,
    abs_total: i128,
    sq_total: i128,
    min: i16,
    max: i16,
}

impl Stats {
    pub fn update(&mut self, value: i16) {
        self.count += 1;

        self.total += value as i128;
        self.abs_total += value.abs() as i128;
        self.sq_total += (value as i128) * (value as i128);

        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let count = self.count as f64;
        let mean = self.total as f64 / count;
        let abs_mean = self.abs_total as f64 / count;
        let variance = (self.sq_total as f64 / count) - (mean * mean);
        let stddev = variance.sqrt();

        writeln!(f, "Count: {}", self.count)?;
        writeln!(f, "Mean: {:.2}", mean)?;
        writeln!(f, "Abs Mean: {:.2}", abs_mean)?;
        writeln!(f, "Stddev: {:.2}", stddev)?;
        writeln!(f, "Min: {}", self.min)?;
        writeln!(f, "Max: {}", self.max)?;

        Ok(())
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            total: 0,
            count: 0,
            abs_total: 0,
            sq_total: 0,
            min: i16::MAX,
            max: i16::MIN,
        }
    }
}
