use std::{env, fs::File, io::Write, path::Path};

mod attacks;
mod magics;
mod maps;

macro_rules! write_map {
    ($f:ident, $name:tt, $type:tt, $items:expr) => {
        let size = $items.len();

        writeln!($f, "pub static {}: [{}; {}] = [", $name, $type, size).unwrap();
        for item in $items {
            write!($f, "{},", item).unwrap();
        }
        writeln!($f, "];").unwrap();
    };
}

fn main() {
    let dir = env::var("OUT_DIR").unwrap();
    let path = Path::new(&dir).join("lookup.rs");
    let mut f = File::create(&path).unwrap();

    write_map!(f, "KING_MAP", "u64", maps::generate_king_map());

    write_map!(f, "ROOK_MAP", "u64", maps::generate_rook_map());
    write_map!(f, "BISHOP_MAP", "u64", maps::generate_bishop_map());

    write_map!(f, "ROOK_MAGICS", "MagicEntry", magics::ROOK_MAGICS);
    write_map!(f, "BISHOP_MAGICS", "MagicEntry", magics::BISHOP_MAGICS);

    writeln!(f, "pub struct MagicEntry {{ pub mask: u64, pub magic: u64, pub shift: u32, pub offset: u32 }}").unwrap();
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
