#![allow(unsafe_op_in_unsafe_fn)]
#![warn(clippy::large_types_passed_by_value)]
#![warn(clippy::trivially_copy_pass_by_ref)]
#![warn(clippy::redundant_clone)]
#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_imports))]

mod board;
mod evaluation;
mod history;
mod lookup;
mod misc;
mod movepick;
mod nnue;
mod numa;
mod parameters;
mod search;
mod setwise;
mod stack;
mod thread;
mod threadpool;
mod time;
mod transposition;
mod types;

mod tools;

#[cfg(not(target_arch = "wasm32"))]
mod uci;

#[cfg(feature = "syzygy")]
mod tb;

#[cfg(feature = "syzygy")]
#[allow(warnings)]
mod bindings;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub fn run(buffer: std::collections::VecDeque<String>) {
    lookup::initialize();
    nnue::initialize();
    uci::message_loop(buffer);
}
