use std::sync::Arc;
use std::sync::atomic::Ordering;

use js_sys::Function as JsFunction;
use wasm_bindgen::prelude::*;

use crate::{
    board::{Board, NullBoardObserver},
    search::Report,
    thread::SharedContext,
    threadpool::ThreadPool,
    time::{Limits, TimeManager},
};

#[wasm_bindgen]
pub struct Engine {
    shared: Arc<SharedContext>,
    threads: ThreadPool,
    board: Board,
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        crate::lookup::initialize();
        crate::nnue::initialize();

        let shared = Arc::new(SharedContext::default());
        let threads = ThreadPool::new(shared.clone());
        Self { shared, threads, board: Board::starting_position() }
    }

    pub fn set_position(&mut self, fen: &str) {
        self.board = Board::from_fen(fen).unwrap_or_else(|_| Board::starting_position());
    }

    pub fn make_move(&mut self, uci_move: &str) {
        let moves = self.board.generate_all_moves();
        if let Some(mv) = moves.iter().map(|e| e.mv).find(|m| m.to_uci(&self.board) == uci_move) {
            self.board.make_move(mv, &mut NullBoardObserver);
        }
    }

    pub fn set_threads(&mut self, n: u32) {
        self.threads.set_count(n as usize);
    }

    pub fn set_dispatch(&self, dispatch: Option<JsFunction>) {
        crate::thread::WASM_DISPATCH.with(|d| *d.borrow_mut() = dispatch);
    }

    pub fn go_uci(&mut self, depth: u32, nodes: u32, multi_pv: u32, on_info: Option<JsFunction>) {
        let multi_pv = (multi_pv as usize).max(1);
        let limits = if nodes > 0 {
            Limits::Nodes(nodes as u64)
        } else if depth > 0 {
            Limits::Depth(depth as i32)
        } else {
            Limits::Infinite
        };
        self.run_search(limits, multi_pv, on_info);
    }

    pub fn go_movetime(&mut self, ms: u32, multi_pv: u32, on_info: Option<JsFunction>) {
        self.run_search(Limits::Time(ms as u64), (multi_pv as usize).max(1), on_info);
    }

    pub fn take_output(&mut self) -> String {
        self.threads.vector[0].writer.take()
    }

    fn run_search(&mut self, limits: Limits, multi_pv: usize, on_info: Option<JsFunction>) {
        crate::thread::WASM_CALLBACK.with(|c| *c.borrow_mut() = on_info);
        let tm = TimeManager::new(limits, self.board.fullmove_number(), 0);
        self.threads.execute_searches(tm, Report::Full, multi_pv, &self.board, &self.shared);
        crate::thread::WASM_CALLBACK.with(|c| *c.borrow_mut() = None);
    }

    pub fn last_bestmove(&self) -> String {
        if self.threads[0].root_moves.is_empty() {
            "(none)".to_string()
        } else {
            self.threads[0].root_moves[0].mv.to_uci(&self.board)
        }
    }

    pub fn last_score(&self) -> i32 {
        if self.threads[0].root_moves.is_empty() { 0 } else { self.threads[0].root_moves[0].score }
    }

    pub fn last_depth(&self) -> i32 {
        self.threads[0].completed_depth
    }

    pub fn fen(&self) -> String {
        self.board.to_fen()
    }

    pub fn evaluate(&mut self) -> i32 {
        self.threads.main_thread().nnue.full_refresh(&self.board);
        self.threads.main_thread().nnue.evaluate(&self.board)
    }

    pub fn last_nodes(&self) -> u64 {
        self.shared.nodes.aggregate()
    }

    pub fn reset(&mut self) {
        self.threads.clear();
        self.shared.tt.clear(1);
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
pub fn run_helper_thread(ptr: u32, thread_count: u32) {
    let t = unsafe { &mut *(ptr as *mut crate::thread::ThreadData) };
    crate::search::start(t, Report::None, thread_count as usize);
    crate::thread::WORKERS_REMAINING.fetch_sub(1, Ordering::Release);
}
