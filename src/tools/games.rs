use std::{
    fs::File,
    io::{BufReader, Read},
};

use pgn_lexer::parser::{PGNTokenIterator, Token};

pub fn count_games(input: &str) {
    let files = std::fs::read_dir(input)
        .unwrap()
        .flatten()
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    println!("Found {} files to process", files.len());

    let mut total_games = 0;

    for file in files {
        let file = File::open(file).unwrap();
        let uncompressed = bzip2::read::BzDecoder::new(file);

        let bytes = BufReader::new(uncompressed).bytes().flatten().collect::<Vec<_>>();

        let mut parser = PGNTokenIterator::new(&bytes);

        while let Some(token) = parser.next() {
            if let Token::Result(_) = token {
                total_games += 1;

                if total_games % 100_000 == 0 {
                    println!("Processed {} games...", total_games);
                }
            }
        }
    }

    println!("Total games found: {}", total_games);
}
