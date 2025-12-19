#![allow(clippy::manual_is_multiple_of)]
#![allow(clippy::if_same_then_else)]

mod board;
mod evaluation;
mod history;
mod lookup;
mod misc;
mod movepick;
mod nnue;
mod parameters;
mod search;
mod stack;
mod tb;
mod thread;
mod threadpool;
mod time;
mod tools;
mod transposition;
mod types;
mod uci;

#[allow(warnings)]
mod bindings;

fn main() {
    lookup::init();
    nnue::initialize();
    nnue::init_parameters();

    match std::env::args().nth(1).as_deref() {
        Some("bench") => tools::bench::<false>(None),
        _ => uci::message_loop(),
    }
}

pub mod numa {
    use libc::{c_int, c_ulong};

    #[link(name = "numa")]
    extern "C" {
        pub fn numa_num_configured_nodes() -> c_int;

        pub fn numa_node_of_cpu(cpu: c_int) -> c_int;

        pub fn numa_run_on_node(node: c_int) -> c_int;
    }

    pub const MPOL_BIND: c_int = 2;

    extern "C" {
        pub fn set_mempolicy(mode: c_int, nodemask: *const c_ulong, maxnode: c_ulong) -> c_int;
    }

    pub fn node_mask(node: usize) -> [c_ulong; 16] {
        let mut mask = [0; 16];
        let word = node / (std::mem::size_of::<c_ulong>() * 8);
        let bit = node % (std::mem::size_of::<c_ulong>() * 8);
        if word < mask.len() {
            mask[word] = 1 << bit;
        }
        mask
    }
}
