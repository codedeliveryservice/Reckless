use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

mod attacks;
mod magics;
mod maps;

fn main() {
    let dir = env::var("OUT_DIR").unwrap();
    let path = Path::new(&dir).join("lookup.rs");
    let out = File::create(path).unwrap();
    write(BufWriter::new(out)).unwrap();
}

fn write(mut buf: BufWriter<File>) -> Result<(), std::io::Error> {
    macro_rules! write_map {
        ($name:tt, $type:tt, $items:expr) => {
            writeln!(buf, "static {}: [{}; {}] = {:?};", $name, $type, $items.len(), $items)?;
        };
    }

    write_map!("KING_MAP", "u64", maps::generate_king_map());
    write_map!("KNIGHT_MAP", "u64", maps::generate_knight_map());

    write_map!("WHITE_PAWN_MAP", "u64", maps::generate_white_pawn_map());
    write_map!("BLACK_PAWN_MAP", "u64", maps::generate_black_pawn_map());

    write_map!("ROOK_MAP", "u64", maps::generate_rook_map());
    write_map!("BISHOP_MAP", "u64", maps::generate_bishop_map());

    write_map!("ROOK_MAGICS", "MagicEntry", magics::ROOK_MAGICS);
    write_map!("BISHOP_MAGICS", "MagicEntry", magics::BISHOP_MAGICS);

    writeln!(
        buf,
        "struct MagicEntry {{ pub mask: u64, pub magic: u64, pub shift: u32, pub offset: u32 }}"
    )
}
