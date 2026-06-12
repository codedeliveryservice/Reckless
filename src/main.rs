#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let buffer = std::env::args().skip(1).collect();
    reckless::run(buffer);
}
