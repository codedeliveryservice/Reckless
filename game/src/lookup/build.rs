use std::{env, fs::File, io::Write, path::Path};

mod attacks;
mod magics;
mod maps;

macro_rules! write_map {
    ($f:ident, $name:tt, $type:tt, $items:expr) => {
        let size = $items.len();

        writeln!($f, "pub const {}: [{}; {}] = [", $name, $type, size).unwrap();
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
}
