use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    fs::File,
    hash::{Hash, Hasher},
    io::{BufReader, BufWriter},
    path::Path,
};

use crate::tools::{BinpackReader, BinpackWriter};

pub fn deduplicate(input: String, output: String) {
    assert!(input != output, "Deduplication in-place is not supported");

    println!("Deduplicating: {input} -> {output}...");

    let file = File::open(input).unwrap();
    let mut reader = BinpackReader::new(BufReader::new(file));

    let output_path = Path::new(&output);
    let output_file = File::create(output_path).unwrap();
    let mut writer = BinpackWriter::new(BufWriter::new(output_file));

    let mut seen = HashSet::new();
    let mut games_processed = 0;
    let mut games_written = 0;
    let mut duplicates = 0;

    while let Some(bytes) = reader.next_bytes() {
        games_processed += 1;

        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();

        if seen.insert(hash) {
            writer.write_bytes(&bytes);
            games_written += 1;
        } else {
            duplicates += 1;
        }

        if games_processed % (1024 * 128) == 0 {
            println!("Processed {games_processed} games ({games_written} unique, {duplicates} duplicates)");
        }
    }

    println!();
    println!("Games processed: {games_processed}");
    println!("Games written: {games_written}");
    println!("Duplicates found: {duplicates} ({:.4}%)", (duplicates as f64 / games_processed as f64) * 100.0);
}
