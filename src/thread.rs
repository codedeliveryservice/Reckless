use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Condvar, Mutex,
};

use crate::{
    board::Board,
    history::{ContinuationCorrectionHistory, ContinuationHistory, CorrectionHistory, NoisyHistory, QuietHistory},
    nnue::Network,
    search::{self, Report},
    stack::Stack,
    time::{Limits, TimeManager},
    transposition::TranspositionTable,
    types::{is_decisive, is_loss, is_win, normalize_to_cp, Color, Move, Score, MAX_MOVES, MAX_PLY},
};

pub struct ThreadData<'a> {
    pub tt: &'a TranspositionTable,
    pub stop: &'a AtomicBool,
    pub nodes: AtomicCounter<'a>,
    pub tb_hits: AtomicCounter<'a>,
    pub board: Board,
    pub time_manager: TimeManager,
    pub stack: Stack,
    pub nnue: Network,
    pub root_moves: Vec<RootMove>,
    pub pv: PrincipalVariationTable,
    pub noisy_history: NoisyHistory,
    pub quiet_history: QuietHistory,
    pub continuation_history: ContinuationHistory,
    pub pawn_corrhist: CorrectionHistory,
    pub minor_corrhist: CorrectionHistory,
    pub major_corrhist: CorrectionHistory,
    pub non_pawn_corrhist: [CorrectionHistory; 2],
    pub continuation_corrhist: ContinuationCorrectionHistory,
    pub lmr: LmrTable,
    pub optimism: [i32; 2],
    pub stopped: bool,
    pub root_depth: i32,
    pub root_delta: i32,
    pub sel_depth: i32,
    pub completed_depth: i32,
    pub ply: usize,
    pub nmp_min_ply: i32,
    pub previous_best_score: i32,
}

impl<'a> ThreadData<'a> {
    pub fn new(tt: &'a TranspositionTable, stop: &'a AtomicBool, nodes: &'a AtomicU64, tb_hits: &'a AtomicU64) -> Self {
        Self {
            tt,
            stop,
            nodes: AtomicCounter::new(nodes),
            tb_hits: AtomicCounter::new(tb_hits),
            board: Board::starting_position(),
            time_manager: TimeManager::new(Limits::Infinite, 0, 0),
            stack: Stack::default(),
            nnue: Network::default(),
            root_moves: Vec::new(),
            pv: PrincipalVariationTable::default(),
            noisy_history: NoisyHistory::default(),
            quiet_history: QuietHistory::default(),
            continuation_history: ContinuationHistory::default(),
            pawn_corrhist: CorrectionHistory::default(),
            minor_corrhist: CorrectionHistory::default(),
            major_corrhist: CorrectionHistory::default(),
            non_pawn_corrhist: [CorrectionHistory::default(), CorrectionHistory::default()],
            continuation_corrhist: ContinuationCorrectionHistory::default(),
            lmr: LmrTable::default(),
            optimism: [0; 2],
            stopped: false,
            root_depth: 0,
            root_delta: 0,
            sel_depth: 0,
            completed_depth: 0,
            ply: 0,
            nmp_min_ply: 0,
            previous_best_score: 0,
        }
    }

    pub fn get_stop(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub fn conthist(&self, index: usize, mv: Move) -> i32 {
        if self.ply < index || self.stack[self.ply - index].mv.is_null() {
            return 0;
        }

        let piece = self.board.piece_on(mv.from());
        let sq = mv.to();
        self.continuation_history.get(self.stack[self.ply - index].conthist, piece, sq)
    }

    pub fn print_uci_info(&self, depth: i32) {
        let elapsed = self.time_manager.elapsed();
        let nps = self.nodes.global() as f64 / elapsed.as_secs_f64();
        let ms = elapsed.as_millis();

        let root_move = &self.root_moves[0];
        let score = if root_move.score == -Score::INFINITE { root_move.display_score } else { root_move.score };

        let score = if score.abs() < Score::TB_WIN_IN_MAX {
            format!("cp {}", normalize_to_cp(score, &self.board))
        } else if score.abs() <= Score::TB_WIN {
            let ply = Score::TB_WIN - score.abs();
            let cp_score = 20_000 - ply;
            format!("cp {}", if score.is_positive() { cp_score } else { -cp_score })
        } else {
            let mate = (Score::MATE - score.abs() + if score.is_positive() { 1 } else { 0 }) / 2;
            format!("mate {}", if score.is_positive() { mate } else { -mate })
        };

        let score = if root_move.upperbound {
            format!("{score} upperbound")
        } else if root_move.lowerbound {
            format!("{score} lowerbound")
        } else {
            score
        };

        print!(
            "info depth {depth} seldepth {} score {score} nodes {} time {ms} nps {nps:.0} hashfull {} tbhits {} pv",
            root_move.sel_depth,
            self.nodes.global(),
            self.tt.hashfull(),
            self.tb_hits.global(),
        );

        for mv in self.pv.line() {
            print!(" {}", mv.to_uci(&self.board));
        }

        if self.pv.line().is_empty() {
            print!(" {}", self.pv.best_move().to_uci(&self.board));
        }

        println!();
    }
}

#[derive(Clone)]
pub struct RootMove {
    pub mv: Move,
    pub score: i32,
    pub display_score: i32,
    pub upperbound: bool,
    pub lowerbound: bool,
    pub sel_depth: i32,
    pub nodes: u64,
}

#[derive(Clone)]
pub struct PrincipalVariationTable {
    table: [[Move; MAX_PLY + 1]; MAX_PLY + 1],
    len: [usize; MAX_PLY + 1],
}

impl PrincipalVariationTable {
    pub const fn best_move(&self) -> Move {
        self.table[0][0]
    }

    pub fn line(&self) -> &[Move] {
        &self.table[0][..self.len[0]]
    }

    pub fn clear(&mut self, ply: usize) {
        self.len[ply] = 0;
    }

    pub fn update(&mut self, ply: usize, mv: Move) {
        self.table[ply][0] = mv;
        self.len[ply] = self.len[ply + 1] + 1;

        for i in 0..self.len[ply + 1] {
            self.table[ply][i + 1] = self.table[ply + 1][i];
        }
    }
}

impl Default for PrincipalVariationTable {
    fn default() -> Self {
        Self {
            table: [[Move::NULL; MAX_PLY + 1]; MAX_PLY + 1],
            len: [0; MAX_PLY + 1],
        }
    }
}

pub struct LmrTable {
    table: Box<[[i32; MAX_MOVES + 1]]>,
}

impl LmrTable {
    pub const fn reduction(&self, depth: i32, move_count: i32) -> i32 {
        self.table[depth as usize][move_count as usize]
    }
}

impl Default for LmrTable {
    fn default() -> Self {
        let mut table = vec![[0; MAX_MOVES + 1]; MAX_MOVES + 1].into_boxed_slice();

        for depth in 1..MAX_MOVES {
            for move_count in 1..MAX_MOVES {
                let reduction = 977.5506 + 443.8557 * (depth as f32).ln() * (move_count as f32).ln();
                table[depth][move_count] = reduction as i32;
            }
        }

        Self { table }
    }
}

pub struct AtomicCounter<'a> {
    buffer: u64,
    local: u64,
    global: &'a AtomicU64,
}

impl<'a> AtomicCounter<'a> {
    pub const fn new(global: &'a AtomicU64) -> Self {
        Self { buffer: 0, local: 0, global }
    }

    pub const fn local(&self) -> u64 {
        self.local + self.buffer
    }

    pub fn global(&self) -> u64 {
        self.buffer + self.global.load(Ordering::Relaxed)
    }

    pub fn increment(&mut self) {
        const BUFFER_SIZE: u64 = 2048;

        self.buffer += 1;
        if self.buffer >= BUFFER_SIZE {
            self.flush();
        }
    }

    pub fn clear(&mut self) {
        self.local = 0;
        self.buffer = 0;
        self.global.store(0, Ordering::Relaxed);
    }

    pub fn flush(&mut self) {
        self.local += self.buffer;
        self.global.fetch_add(self.buffer, Ordering::Relaxed);
        self.buffer = 0;
    }
}

pub struct ThreadPool {
    context: Arc<(Mutex<Vec<()>>, Condvar)>,
    senders: Vec<std::sync::mpsc::Sender<Message>>,
    handlers: Vec<std::thread::JoinHandle<()>>,
}

enum Message {
    Search(TimeManager, std::sync::mpsc::Sender<SearchResult>),
    SetBoard(Board),
    Terminate,
    Clear,
    GetBoardInfo(std::sync::mpsc::Sender<(Color, usize)>),
}

struct SearchResult {
    root_moves: Vec<RootMove>,
    pv: PrincipalVariationTable,
    completed_depth: i32,
}

impl ThreadPool {
    pub fn new() -> Self {
        Self {
            context: Arc::new((Mutex::new(Vec::new()), Condvar::new())),
            senders: Vec::new(),
            handlers: Vec::new(),
        }
    }

    pub fn resize(
        &mut self, requested: usize, tt: &'static TranspositionTable, stop: &'static AtomicBool,
        nodes: &'static AtomicU64, tb_hits: &'static AtomicU64,
    ) {
        while self.handlers.len() > requested {
            if let Some(tx) = self.senders.pop() {
                tx.send(Message::Terminate).unwrap();
            }
            if let Some(handler) = self.handlers.pop() {
                handler.join().unwrap();
            }
        }

        while self.handlers.len() < requested {
            let (tx, rx) = std::sync::mpsc::channel();

            let id = self.handlers.len();
            let context = self.context.clone();

            let handler = std::thread::spawn(move || {
                let mut td = ThreadData::new(tt, stop, nodes, tb_hits);

                loop {
                    match rx.recv() {
                        Ok(Message::Search(time_manager, reply)) => {
                            let (lock, cvar) = &*context;

                            if id == 0 {
                                td.time_manager = time_manager;
                            }

                            search::start(&mut td, if id == 0 { Report::Minimal } else { Report::None });
                            stop.store(true, Ordering::Relaxed);

                            lock.lock().unwrap().push(());
                            cvar.notify_one();

                            reply
                                .send(SearchResult {
                                    root_moves: td.root_moves.clone(),
                                    pv: td.pv.clone(),
                                    completed_depth: td.completed_depth,
                                })
                                .unwrap();
                        }
                        Ok(Message::Clear) => {
                            td = ThreadData::new(tt, stop, nodes, tb_hits);
                        }
                        Ok(Message::SetBoard(board)) => {
                            td.board = board;
                        }
                        Ok(Message::GetBoardInfo(reply)) => {
                            reply.send((td.board.side_to_move(), td.board.fullmove_number())).unwrap();
                        }
                        Ok(Message::Terminate) | Err(_) => break,
                    }
                }
            });

            self.senders.push(tx);
            self.handlers.push(handler);
        }
    }

    pub fn search(&mut self, time_manager: TimeManager) {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();

        for sender in self.senders.iter() {
            sender.send(Message::Search(time_manager.clone(), reply_tx.clone())).unwrap();
        }

        let (lock, cvar) = &*self.context;
        let mut guard = lock.lock().unwrap();
        while guard.len() < self.handlers.len() {
            guard = cvar.wait(guard).unwrap();
        }
        std::mem::take(&mut *guard);

        let results = reply_rx.iter().take(self.handlers.len()).collect::<Vec<_>>();

        let min_score = results.iter().map(|v| v.root_moves[0].score).min().unwrap();
        let vote_value = |result: &SearchResult| (result.root_moves[0].score - min_score + 10) * result.completed_depth;

        let mut votes = vec![0; 4096];
        for result in results.iter() {
            votes[result.pv.best_move().encoded()] += vote_value(result);
        }

        let mut best = 0;

        for current in 1..results.len() {
            let is_better_candidate = || -> bool {
                let best = &results[best];
                let current = &results[current];

                if is_decisive(best.root_moves[0].score) {
                    return current.root_moves[0].score > best.root_moves[0].score;
                }

                if is_win(current.root_moves[0].score) {
                    return true;
                }

                let best_vote = votes[best.pv.best_move().encoded()];
                let current_vote = votes[current.pv.best_move().encoded()];

                !is_loss(current.root_moves[0].score)
                    && (current_vote > best_vote
                        || (current_vote == best_vote && vote_value(current) > vote_value(best)))
            };

            if is_better_candidate() {
                best = current;
            }
        }

        // TODO: This breaks (D)FRC
        println!("bestmove {}", results[best].pv.best_move().to_uci(&Board::default()));
    }

    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    pub fn clear(&self) {
        for sender in self.senders.iter() {
            sender.send(Message::Clear).unwrap();
        }
    }

    pub fn set_board(&self, board: &Board) {
        for sender in self.senders.iter() {
            sender.send(Message::SetBoard(board.clone())).unwrap();
        }
    }

    pub fn get_board_info(&self) -> (Color, usize) {
        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
        self.senders[0].send(Message::GetBoardInfo(reply_tx)).unwrap();
        reply_rx.recv().unwrap()
    }
}
