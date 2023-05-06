use std::time::Duration;

use game::MAX_SEARCH_DEPTH;

#[derive(Debug, PartialEq)]
pub enum TimeControl {
    Infinite,
    FixedDepth(usize),
    FixedTime(u64),
    Incremental(u64, u64),
    Tournament(u64, u64, u64),
}

/// The default number of moves left.
/// This results in slower play at the beginning and faster towards the end.
const MOVES_TO_GO: u64 = 25;

/// Safe margin for move time overhead.
const TIME_MARGIN_MS: u64 = 25;

impl TimeControl {
    /// Returns the maximum `depth` for the current `TimeControl`. The depth value
    /// can take the specified value or infinity if no depth limit is specified.
    #[inline(always)]
    pub fn get_max_depth(&self) -> usize {
        match self {
            Self::FixedDepth(depth) => *depth,
            _ => MAX_SEARCH_DEPTH,
        }
    }

    /// Returns `true` if the time has expired.
    #[inline(always)]
    pub fn is_time_over(&self, elapsed: Duration) -> bool {
        let spent = elapsed.as_millis() as u64 + TIME_MARGIN_MS;

        match *self {
            Self::FixedTime(time) => spent >= time,
            Self::Incremental(main, inc) => spent >= main / MOVES_TO_GO + inc,
            Self::Tournament(main, inc, moves) => spent >= main / moves + inc,
            _ => false,
        }
    }
}
