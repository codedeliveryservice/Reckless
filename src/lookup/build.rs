use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

mod attacks;
mod magics;
mod maps;
mod random;

macro_rules! write_map {
    ($f:ident, $name:tt, $type:tt, $items:expr) => {
        let size = $items.len();
        writeln!($f, "static {}: [{}; {}] = [", $name, $type, size)?;
        for item in $items {
            write!($f, "{},", item)?;
        }
        writeln!($f, "];")?;
    };
}

fn main() {
    write_lookup(get_buf("lookup.rs")).unwrap();
    write_zobrist(get_buf("zobrist.rs")).unwrap();
}

fn write_lookup(mut f: BufWriter<File>) -> Result<(), std::io::Error> {
    write_map!(f, "KING_MAP", "u64", maps::generate_king_map());
    write_map!(f, "KNIGHT_MAP", "u64", maps::generate_knight_map());

    write_map!(f, "WHITE_PAWN_MAP", "u64", maps::generate_white_pawn_map());
    write_map!(f, "BLACK_PAWN_MAP", "u64", maps::generate_black_pawn_map());

    write_map!(f, "ROOK_MAP", "u64", maps::generate_rook_map());
    write_map!(f, "BISHOP_MAP", "u64", maps::generate_bishop_map());

    write_map!(f, "ROOK_MAGICS", "MagicEntry", magics::ROOK_MAGICS);
    write_map!(f, "BISHOP_MAGICS", "MagicEntry", magics::BISHOP_MAGICS);

    writeln!(
        f,
        "struct MagicEntry {{ pub mask: u64, pub magic: u64, pub shift: u32, pub offset: u32 }}"
    )
}

fn write_zobrist(mut f: BufWriter<File>) -> Result<(), std::io::Error> {
    let mut rng = random::Random::new();

    write_map!(f, "CASTLING_KEYS", "u64", rng.array::<16>());
    write_map!(f, "EN_PASSANT_KEYS", "u64", rng.array::<64>());

    writeln!(f, "const SIDE_KEY: u64 = {};", rng.next_u64())?;

    // [color 0..2][piece 0..6][square 0..64]
    writeln!(f, "const PIECE_KEYS: [[[u64; 64]; 6]; 2] = [")?;
    for _ in 0..2 {
        write!(f, "[")?;
        for _ in 0..6 {
            write!(f, "{:?},", rng.array::<64>())?;
        }
        write!(f, "],")?;
    }
    writeln!(f, "];")
}

fn get_buf(file: &str) -> BufWriter<File> {
    let dir = env::var("OUT_DIR").unwrap();
    let path = Path::new(&dir).join(file);
    let out = File::create(path).unwrap();
    BufWriter::new(out)
}

impl std::fmt::Display for magics::MagicEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "MagicEntry {{ mask: {}, magic: {}, shift: {}, offset: {} }}",
            self.mask, self.magic, self.shift, self.offset
        )
    }
}
