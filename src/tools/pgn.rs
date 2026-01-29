use std::{
    fs::File,
    io::{BufReader, BufWriter, Read},
    sync::{Arc, Mutex},
};

use indicatif::{ProgressBar, ProgressStyle};
use pgn_lexer::parser::{PGNTokenIterator, Token};
use shakmaty::{fen::Fen, san::San, CastlingMode, Chess, Position};

use crate::{board::Board, nnue::Network, tools::BinpackWriter};

pub fn convert_pgns(input: &str, output: &str, threads: usize) {
    println!("Converting PGNs from '{input}' to '{output}' using {threads} threads [adversarial=full]");

    let mut handlers = Vec::new();

    std::fs::create_dir(output).unwrap();

    let mut files = std::fs::read_dir(input)
        .unwrap()
        .flatten()
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    files.sort_by_key(|f| std::fs::metadata(f).unwrap().len());
    files.reverse();

    let bar = ProgressBar::new(files.len() as u64).with_style(
        ProgressStyle::with_template("{spinner:.green} [{bar:40}] {pos}/{len} ({eta}) {percent}%")
            .unwrap()
            .progress_chars("=> "),
    );

    let progress = Arc::new(Mutex::new(bar));

    let mut chunks = vec![Vec::new(); threads];
    for (i, item) in files.into_iter().enumerate() {
        chunks[i % threads].push(item);
    }

    for (index, chunk) in chunks.iter().enumerate() {
        let chunk = chunk.to_vec();
        let output = output.to_string();
        let progress = progress.clone();

        let handler = std::thread::spawn(move || {
            let buf = BufWriter::new(File::create(format!("{output}/chunk_{index}.rbinpack")).unwrap());
            let mut writer = BinpackWriter::new(buf);
            let mut nnue = Network::default();

            for file_name in chunk {
                convert_pgn(&file_name, &mut writer, &mut nnue);
                progress.lock().unwrap().inc(1);
            }
        });
        handlers.push(handler);
    }

    for handler in handlers {
        handler.join().unwrap();
    }

    std::process::exit(0);
}

pub fn convert_pgn(file_name: &str, writer: &mut BinpackWriter, nnue: &mut Network) {
    let file = File::open(file_name).unwrap();
    let uncompressed = bzip2::read::BzDecoder::new(file);
    let bytes = BufReader::new(uncompressed).bytes().flatten().collect::<Vec<_>>();

    let mut parser = PGNTokenIterator::new(&bytes);

    let mut position = Chess::default();
    let mut start_board = Board::default();

    let mut internal_board = Board::default();
    let mut internal_entries = Vec::new();

    let mut skip_game = false;

    while let Some(token) = parser.next() {
        match token {
            Token::TagSymbol(bytes) if bytes == b"FEN" => {
                let fen_bytes = match parser.next() {
                    Some(Token::TagString(v)) => v,
                    _ => panic!(),
                };

                start_board = Board::from_fen(&String::from_utf8_lossy(fen_bytes)).unwrap();

                internal_board = start_board.clone();
                internal_entries.clear();

                position = Fen::from_ascii(fen_bytes).unwrap().into_position(CastlingMode::Standard).unwrap();
            }
            Token::TagSymbol(bytes) if bytes == b"Termination" => {
                let reason = match parser.next() {
                    Some(Token::TagString(v)) => String::from_utf8_lossy(v).to_string(),
                    _ => panic!(),
                };

                if reason == "abandoned" {
                    skip_game = true;
                }
            }
            Token::Result(bytes) => {
                let result = match bytes {
                    b"1-0" => 2,
                    b"1/2-1/2" => 1,
                    b"0-1" => 0,
                    _ => panic!("Unexpected result: {:?}", String::from_utf8_lossy(&bytes)),
                };

                if !skip_game {
                    writer.write(&start_board, result, &internal_entries);
                }
                skip_game = false;
            }
            _ => (),
        }

        if let Token::Move(bytes) = token {
            nnue.full_refresh(&internal_board);            
            let score = nnue.evaluate(&internal_board);

            let san = San::from_ascii(bytes).unwrap();
            let mv = san.to_move(&position).unwrap();
            let uci_move = mv.to_uci(CastlingMode::Standard).to_string();

            let internal_move = internal_board
                .generate_all_moves()
                .iter()
                .map(|entry| entry.mv)
                .find(|m| m.to_uci(&internal_board) == uci_move)
                .unwrap();

            internal_entries.push((internal_move, score.try_into().unwrap()));
            internal_board.make_move(internal_move, |_, _, _, _| ());
            position.play_unchecked(&mv);
        }
    }
}
