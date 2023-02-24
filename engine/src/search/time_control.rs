use std::time::{Duration, Instant};

pub struct TimeControl {
    pub exactly: bool,
    pub max_depth: usize,
    pub stop_time: Instant,
}

impl TimeControl {
    /// Generates a new `TimeControl`.
    pub fn generate(
        main: Option<u32>,
        inc: Option<u32>,
        moves: Option<u32>,
        movetime: Option<u32>,
        depth: Option<usize>,
    ) -> Self {
        Self {
            exactly: movetime.is_some(),
            max_depth: get_max_depth(depth),
            stop_time: get_stop_time(main, inc, moves, movetime, depth),
        }
    }

    /// Returns `true` if the time has expired.
    #[inline(always)]
    pub fn is_time_over(&self) -> bool {
        Instant::now() >= self.stop_time
    }
}

/// The default number of moves left.
/// This results in slower play at the beginning and faster towards the end.
const MOVES_TO_GO: u32 = 25;

/// Safe margin for move time overhead.
const TIME_MARGIN_MS: u32 = 25;

/// Represents a pseudo infinite time of ≈1 year.
const INFINITE_TIME_MS: u64 = 1000 * 60 * 60 * 24 * 365;

/// Diving to a depth of 64 will take years, so it's considered infinite.
const INFINITE_DEPTH: usize = 64;

/// Returns the maximum `depth` for the current `TimeControl`. The depth value
/// can take the specified value or infinity if no depth limit is specified.
#[inline(always)]
fn get_max_depth(depth: Option<usize>) -> usize {
    match depth {
        Some(value) => value,
        None => INFINITE_DEPTH,
    }
}

/// Returns `Instant` representing the expiration of the search time.
#[inline(always)]
fn get_stop_time(
    main: Option<u32>,
    inc: Option<u32>,
    moves: Option<u32>,
    movetime: Option<u32>,
    depth: Option<usize>,
) -> Instant {
    if depth.is_some() {
        return Instant::now() + Duration::from_millis(INFINITE_TIME_MS);
    }

    if let Some(time) = movetime {
        return Instant::now() + Duration::from_millis(indemnify(time) as u64);
    }

    let mut total_ms = 0;

    if let Some(time) = main {
        total_ms += time / get_moves_to_go(moves);
    }

    if let Some(time) = inc {
        total_ms += time;
    }

    Instant::now() + Duration::from_millis(indemnify(total_ms) as u64)
}

/// Compensate overheads and ensure that the expiration time is legal.
fn indemnify(mut time: u32) -> u32 {
    if time > TIME_MARGIN_MS {
        time -= TIME_MARGIN_MS;
    }

    // Ensure that there is time for at least one search to avoid sending an empty-move
    if time == 0 {
        time = 1;
    }

    time
}

fn get_moves_to_go(moves: Option<u32>) -> u32 {
    match moves {
        // Assume that there are more moves to be played than there actually are
        Some(count) => count + 2,
        None => MOVES_TO_GO,
    }
}
