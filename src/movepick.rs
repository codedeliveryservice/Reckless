use crate::{
    parameters::PIECE_VALUES,
    search::NodeType,
    thread::ThreadData,
    types::{ArrayVec, Move, MoveList, PieceType, MAX_MOVES},
};

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd)]
pub enum Stage {
    HashMove,
    GenerateNoisy,
    GoodNoisy,
    GenerateQuiet,
    Quiet,
    BadNoisy,
}

pub struct MovePicker {
    list: MoveList,
    tt_move: Move,
    threshold: Option<i32>,
    state: fn(&mut MovePicker, &ThreadData, bool, isize) -> Option<Move>,
    bad_noisy: ArrayVec<Move, MAX_MOVES>,
    bad_noisy_idx: usize,
}

impl MovePicker {
    pub const fn new<NODE: NodeType>(tt_move: Move) -> Self {
        Self {
            list: MoveList::new(),
            tt_move,
            threshold: None,
            state: if tt_move.is_some() { hash_move::<NODE> } else { generate_noisy::<NODE> },
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub const fn new_probcut<NODE: NodeType>(threshold: i32) -> Self {
        Self {
            list: MoveList::new(),
            tt_move: Move::NULL,
            threshold: Some(threshold),
            state: generate_noisy::<NODE>,
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub const fn new_qsearch<NODE: NodeType>() -> Self {
        Self {
            list: MoveList::new(),
            tt_move: Move::NULL,
            threshold: None,
            state: generate_noisy::<NODE>,
            bad_noisy: ArrayVec::new(),
            bad_noisy_idx: 0,
        }
    }

    pub fn stage<NODE: NodeType>(&self) -> Stage {
        match self.state as usize {
            x if x == hash_move::<NODE> as usize => Stage::HashMove,
            x if x == generate_noisy::<NODE> as usize => Stage::GenerateNoisy,
            x if x == good_noisy::<NODE> as usize => Stage::GoodNoisy,
            x if x == generate_quiet::<NODE> as usize => Stage::GenerateQuiet,
            x if x == quiet::<NODE> as usize => Stage::Quiet,
            x if x == bad_noisy::<NODE> as usize => Stage::BadNoisy,
            _ => unreachable!(),
        }
    }

    pub fn next(&mut self, td: &ThreadData, skip_quiets: bool, ply: isize) -> Option<Move> {
        (self.state)(self, td, skip_quiets, ply)
    }

    fn find_best_score_index(&self) -> usize {
        let mut best_index = 0;
        let mut best_score = i32::MIN;

        for (index, entry) in self.list.iter().enumerate() {
            if entry.score >= best_score {
                best_index = index;
                best_score = entry.score;
            }
        }

        best_index
    }

    fn score_noisy(&mut self, td: &ThreadData) {
        let threats = td.board.threats();

        for entry in self.list.iter_mut() {
            let mv = entry.mv;

            if mv == self.tt_move {
                entry.score = -32768;
                continue;
            }

            let captured =
                if entry.mv.is_en_passant() { PieceType::Pawn } else { td.board.piece_on(mv.to()).piece_type() };

            entry.score = 16 * PIECE_VALUES[captured]
                + td.noisy_history.get(threats, td.board.moved_piece(mv), mv.to(), captured);
        }
    }

    fn score_quiet(&mut self, td: &ThreadData, ply: isize) {
        let threats = td.board.threats();
        let side = td.board.side_to_move();

        for entry in self.list.iter_mut() {
            let mv = entry.mv;

            if mv == self.tt_move {
                entry.score = -32768;
                continue;
            }

            entry.score = td.quiet_history.get(threats, side, mv)
                + td.conthist(ply, 1, mv)
                + td.conthist(ply, 2, mv)
                + td.conthist(ply, 4, mv)
                + td.conthist(ply, 6, mv);
        }
    }
}

fn hash_move<NODE: NodeType>(mp: &mut MovePicker, td: &ThreadData, skip_quiets: bool, ply: isize) -> Option<Move> {
    mp.state = generate_noisy::<NODE>;

    if td.board.is_pseudo_legal(mp.tt_move) {
        return Some(mp.tt_move);
    }

    (mp.state)(mp, td, skip_quiets, ply)
}

fn generate_noisy<NODE: NodeType>(mp: &mut MovePicker, td: &ThreadData, skip_quiets: bool, ply: isize) -> Option<Move> {
    mp.state = good_noisy::<NODE>;

    td.board.append_noisy_moves(&mut mp.list);
    mp.score_noisy(td);

    (mp.state)(mp, td, skip_quiets, ply)
}

fn good_noisy<NODE: NodeType>(mp: &mut MovePicker, td: &ThreadData, skip_quiets: bool, ply: isize) -> Option<Move> {
    while !mp.list.is_empty() {
        let index = mp.find_best_score_index();
        let entry = &mp.list.remove(index);
        if entry.mv == mp.tt_move {
            continue;
        }

        let threshold = mp.threshold.unwrap_or_else(|| -entry.score / 36 + 119);
        if !td.board.see(entry.mv, threshold) {
            mp.bad_noisy.push(entry.mv);
            continue;
        }

        if NODE::ROOT {
            mp.score_noisy(td);
        }

        return Some(entry.mv);
    }

    mp.state = generate_quiet::<NODE>;

    (mp.state)(mp, td, skip_quiets, ply)
}

fn generate_quiet<NODE: NodeType>(mp: &mut MovePicker, td: &ThreadData, skip_quiets: bool, ply: isize) -> Option<Move> {
    if skip_quiets {
        mp.state = bad_noisy::<NODE>;
    } else {
        mp.state = quiet::<NODE>;

        td.board.append_quiet_moves(&mut mp.list);
        mp.score_quiet(td, ply);
    }

    (mp.state)(mp, td, skip_quiets, ply)
}

fn quiet<NODE: NodeType>(mp: &mut MovePicker, td: &ThreadData, skip_quiets: bool, ply: isize) -> Option<Move> {
    if !skip_quiets {
        while !mp.list.is_empty() {
            let index = mp.find_best_score_index();
            let entry = &mp.list.remove(index);
            if entry.mv == mp.tt_move {
                continue;
            }

            if NODE::ROOT {
                mp.score_quiet(td, ply);
            }

            return Some(entry.mv);
        }
    }

    mp.state = bad_noisy::<NODE>;

    (mp.state)(mp, td, skip_quiets, ply)
}

fn bad_noisy<NODE: NodeType>(mp: &mut MovePicker, _: &ThreadData, _: bool, _: isize) -> Option<Move> {
    while mp.bad_noisy_idx < mp.bad_noisy.len() {
        let mv = mp.bad_noisy[mp.bad_noisy_idx];
        mp.bad_noisy_idx += 1;

        if mv == mp.tt_move {
            continue;
        }

        return Some(mv);
    }

    None
}
