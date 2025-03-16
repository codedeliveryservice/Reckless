use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
};

mod attacks;
mod magics;
mod maps;

const BASE_URL: &str = "https://github.com/codedeliveryservice/RecklessNetworks/raw/main";
const NETWORK_NAME: &str = "v8-89857cf5.nnue";

fn main() {
    generate_model_env();
    generate_attack_maps();

    if !Path::new("networks").join(NETWORK_NAME).exists() && env::var("EVALFILE").is_err() {
        download_network();
    }

    println!("cargo:rerun-if-env-changed=EVALFILE");
    println!("cargo:rerun-if-changed=networks/{NETWORK_NAME}");
}

fn generate_model_env() {
    let mut path = env::var("EVALFILE").map(PathBuf::from).unwrap_or_else(|_| Path::new("networks").join(NETWORK_NAME));

    if path.is_relative() {
        path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    }

    println!("cargo:rustc-env=MODEL={}", path.display());
}

fn generate_attack_maps() {
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

    writeln!(buf, "struct MagicEntry {{ pub mask: u64, pub magic: u64, pub shift: u32, pub offset: u32 }}")
}

fn download_network() {
    let response = Command::new("curl")
        .arg("-sL")
        .arg(format!("{BASE_URL}/{NETWORK_NAME}"))
        .output()
        .expect("Failed to execute `curl`");

    if response.status.success() {
        std::fs::create_dir_all("networks").unwrap();
        std::fs::write(format!("networks/{NETWORK_NAME}"), response.stdout).unwrap();
    } else {
        panic!("Failed to download the network");
    }
}
