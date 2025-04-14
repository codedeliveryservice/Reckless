use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
};

use super::binpack::BinpackReader;
use crate::{
    board::Board,
    types::{Color, PieceType},
};

#[derive(Debug)]
pub struct Filter {
    pub min_ply: usize,
    pub max_ply: usize,
    pub max_score: i16,
}

pub fn convert_to_bullet_format(input: &str, output: &str, filter: Filter) {
    println!("Converting {input} to {output} with filter:");
    println!("{filter:#?}");

    let input = File::open(input).unwrap();
    let output = File::create(output).unwrap();

    let mut reader = BinpackReader::new(BufReader::new(input));
    let mut writer = BufWriter::new(output);

    let mut total = 0;
    let mut filtered = 0;
    let mut wdl = [0; 3];

    while let Some((position, entries)) = reader.next() {
        let mut board = position.to_board();

        for (index, &(mv, score)) in entries.iter().enumerate() {
            let ply = board.fullmove_number * 2 + index;

            if score.abs() <= filter.max_score
                && !board.in_check()
                && !mv.is_capture()
                && !mv.is_promotion()
                && (filter.min_ply..=filter.max_ply).contains(&ply)
            {
                let bullet_format = BulletFormat::new(&board, position.result, score);
                writer.write_all(bullet_format.as_bytes()).unwrap();

                filtered += 1;
                wdl[position.result as usize] += 1;
            }

            board.make_move(mv);
            total += 1;
        }
    }

    writer.flush().unwrap();

    println!("Total:    {total}");
    println!("Filtered: {filtered}");
    println!("Ratio:    {:.2}%", (filtered as f64 / total as f64) * 100.0);
    println!("Win:      {:.2}%", (wdl[0] as f64 / filtered as f64) * 100.0);
    println!("Draw:     {:.2}%", (wdl[1] as f64 / filtered as f64) * 100.0);
    println!("Loss:     {:.2}%", (wdl[2] as f64 / filtered as f64) * 100.0);
}

#[repr(C, packed)]
pub struct BulletFormat {
    occupancies: u64,
    pieces: u128,
    score: i16,
    result: u8,
    our_ksq: u8,
    opp_ksq: u8,
    extra: [u8; 3],
}

struct Occupancy {
    color: u8,
    piece: u8,
    square: u8,
}

impl BulletFormat {
    pub fn new(board: &Board, result: u8, score: i16) -> Self {
        let reverse = board.side_to_move() == Color::Black;

        let mut packed = Vec::new();
        for color in [Color::White, Color::Black] {
            for piece in 0..6 {
                for square in board.of(PieceType::new(piece), color) {
                    packed.push(Occupancy {
                        piece: piece as u8,
                        color: color as u8 ^ reverse as u8,
                        square: square as u8 ^ (reverse as u8 * 56),
                    });
                }
            }
        }

        packed.sort_by_key(|occ| occ.square as usize);

        let mut occupancies = 0;
        let mut pieces = 0;

        for (index, Occupancy { color, piece, square }) in packed.into_iter().enumerate() {
            pieces |= ((color as u128) << 3 | (piece as u128)) << (index * 4);
            occupancies |= 1 << square as usize;
        }

        Self {
            occupancies,
            pieces,
            score,
            our_ksq: (board.king_square(board.side_to_move()) as u8) ^ (reverse as u8 * 56),
            opp_ksq: (board.king_square(!board.side_to_move()) as u8 ^ 56) ^ (reverse as u8 * 56),
            result: if reverse { 2 - result } else { result },
            extra: [0; 3],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        let pointer = self as *const _ as *const u8;
        unsafe { std::slice::from_raw_parts(pointer, std::mem::size_of::<BulletFormat>()) }
    }
}
