//! Search metrics storage and reporting.
//!
//! Search instrumentation has two jobs: count deterministic search-shape events and, in timed
//! builds, attribute exclusive time to named search phases. The search modules only name events and
//! phases through macros. This module owns storage, per-thread merging, timing scopes, and TSV
//! output.
//!
//! The storage is deliberately simpler than general-purpose telemetry. Search updates metrics from
//! the worker thread that owns the corresponding `ThreadData`, and `profilebench` reads metrics
//! only after all workers join. That single-writer/after-join contract lets the hot path use plain
//! per-thread counters instead of atomics.
#![allow(dead_code)]

use std::cell::UnsafeCell;

macro_rules! metric_names {
    (
        $(#[$meta:meta])*
        $visibility:vis enum $name:ident {
            $($variant:ident => $label:literal,)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Copy, Clone, Eq, PartialEq)]
        $visibility enum $name {
            $($variant,)+
        }

        impl $name {
            $visibility const COUNT: usize = [$(stringify!($variant),)+].len();

            const ALL: [Self; Self::COUNT] = [$(Self::$variant,)+];

            const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $label,)+
                }
            }
        }
    };
}

metric_names! {
    /// Search event counters used to compare behavior-equivalent builds.
    pub enum Event {
        RootIteration => "root_iteration",
        DepthCompleted => "depth_completed",
        FullNode => "full_node",
        QsearchNode => "qsearch_node",
        TtProbe => "tt_probe",
        TtHit => "tt_hit",
        TtCutoff => "tt_cutoff",
        TtFullCutoff => "tt_full_cutoff",
        TtQsearchCutoff => "tt_qsearch_cutoff",
        TtWrite => "tt_write",
        TtLowerWrite => "tt_lower_write",
        TtUpperWrite => "tt_upper_write",
        TtExactWrite => "tt_exact_write",
        TtEvalWrite => "tt_eval_write",
        StaticEval => "static_eval",
        TtRawEval => "tt_raw_eval",
        TtScoreAsEval => "tt_score_as_eval",
        CorrectionApplied => "correction_applied",
        TablebaseProbe => "tablebase_probe",
        TablebaseCutoff => "tablebase_cutoff",
        TablebasePvLower => "tablebase_pv_lower",
        TablebasePvUpper => "tablebase_pv_upper",
        TablebaseNoResult => "tablebase_no_result",
        RazorTried => "razor_tried",
        RazorCutoff => "razor_cutoff",
        ReverseFutilityTried => "reverse_futility_tried",
        ReverseFutilityCutoff => "reverse_futility_cutoff",
        NullMoveTried => "null_move_tried",
        NullMoveCutoff => "null_move_cutoff",
        ProbCutTried => "probcut_tried",
        ProbCutCandidate => "probcut_candidate",
        ProbCutQsearchPass => "probcut_qsearch_pass",
        ProbCutConfirmed => "probcut_confirmed",
        ProbCutCutoff => "probcut_cutoff",
        SingularTried => "singular_tried",
        SingularVerified => "singular_verified",
        SingularExtension => "singular_extension",
        SingularDoubleExtension => "singular_double_extension",
        SingularTripleExtension => "singular_triple_extension",
        SingularCutoff => "singular_cutoff",
        SingularNegativeExtension => "singular_negative_extension",
        SingularTtMoveSuppressed => "singular_tt_move_suppressed",
        MoveLoopCandidate => "move_loop_candidate",
        RootMoveSkipped => "root_move_skipped",
        MovePruned => "move_pruned",
        MovePrunedLateMove => "move_pruned_late_move",
        MovePrunedFutility => "move_pruned_futility",
        MovePrunedBadNoisy => "move_pruned_bad_noisy",
        MovePrunedSee => "move_pruned_see",
        ReducedSearch => "reduced_search",
        FullDepthSearch => "full_depth_search",
        PvSearch => "pv_search",
        Research => "research",
        BetaCutoff => "beta_cutoff",
        QsearchStandPatCutoff => "qsearch_stand_pat_cutoff",
        QsearchCandidate => "qsearch_candidate",
        QsearchQuietSkip => "qsearch_quiet_skip",
        QsearchSeePrune => "qsearch_see_prune",
        QsearchBetaCutoff => "qsearch_beta_cutoff",
        QsearchTtWrite => "qsearch_tt_write",
    }
}

metric_names! {
    /// Exclusive timing phases for named search concepts.
    pub enum Phase {
        RootSearch => "root_search",
        FullEntry => "full_entry",
        Proof => "proof",
        EvalSetup => "eval_setup",
        PreMovePruning => "pre_move_pruning",
        Singular => "singular",
        MoveLoop => "move_loop",
        ChildSearch => "child_search",
        Finalization => "finalization",
        QsearchEntry => "qsearch_entry",
        QsearchStandPat => "qsearch_stand_pat",
        QsearchMoveLoop => "qsearch_move_loop",
        QsearchFinalization => "qsearch_finalization",
        NullMove => "null_move",
        ProbCut => "probcut",
        MovePicker => "move_picker",
        CandidatePruning => "candidate_pruning",
        MakeUndo => "make_undo",
        Reduction => "reduction",
        HistoryUpdate => "history_update",
        TtAccess => "tt_access",
    }
}

#[repr(align(64))]
pub struct MetricsShard {
    events: UnsafeCell<[u64; Event::COUNT]>,
    phases: UnsafeCell<[PhaseStats; Phase::COUNT]>,
    stack: UnsafeCell<PhaseStack>,
}

// Safety: each shard is written by the worker whose `ThreadData::id` indexes it. Metrics are read
// or reset only outside active search, after worker joins or before the next search begins.
unsafe impl Sync for MetricsShard {}

impl MetricsShard {
    const fn new() -> Self {
        Self {
            events: UnsafeCell::new([0; Event::COUNT]),
            phases: UnsafeCell::new([const { PhaseStats::new() }; Phase::COUNT]),
            stack: UnsafeCell::new(PhaseStack::new()),
        }
    }

    #[inline]
    fn event(&self, event: Event) {
        unsafe {
            (*self.events.get())[event as usize] += 1;
        }
    }

    #[cfg(feature = "search-metrics-time")]
    #[inline]
    fn enter(&self, phase: Phase) {
        let now = timer::ticks();
        unsafe {
            let phases = &mut *self.phases.get();
            let stack = &mut *self.stack.get();

            if let Some(active) = stack.last_mut() {
                phases[active.phase as usize].ticks += now.saturating_sub(active.started);
            }

            phases[phase as usize].calls += 1;
            stack.push(ActivePhase { phase, started: now });
        }
    }

    #[cfg(feature = "search-metrics-time")]
    #[inline]
    fn exit(&self, phase: Phase) {
        let now = timer::ticks();
        unsafe {
            let phases = &mut *self.phases.get();
            let stack = &mut *self.stack.get();
            let active = stack.pop();
            debug_assert!(active.phase == phase);

            phases[phase as usize].ticks += now.saturating_sub(active.started);

            if let Some(parent) = stack.last_mut() {
                parent.started = now;
            }
        }
    }

    fn reset(&self) {
        unsafe {
            *self.events.get() = [0; Event::COUNT];
            *self.phases.get() = [const { PhaseStats::new() }; Phase::COUNT];
            *self.stack.get() = PhaseStack::new();
        }
    }
}

#[derive(Copy, Clone)]
struct PhaseStats {
    calls: u64,
    ticks: u64,
}

impl PhaseStats {
    const fn new() -> Self {
        Self { calls: 0, ticks: 0 }
    }
}

#[derive(Copy, Clone)]
struct ActivePhase {
    phase: Phase,
    started: u64,
}

struct PhaseStack {
    data: [ActivePhase; 1024],
    len: usize,
}

impl PhaseStack {
    const fn new() -> Self {
        Self {
            data: [ActivePhase { phase: Phase::RootSearch, started: 0 }; 1024],
            len: 0,
        }
    }

    #[inline]
    fn push(&mut self, phase: ActivePhase) {
        debug_assert!(self.len < self.data.len());
        self.data[self.len] = phase;
        self.len += 1;
    }

    #[inline]
    fn pop(&mut self) -> ActivePhase {
        debug_assert!(self.len > 0);
        self.len -= 1;
        self.data[self.len]
    }

    #[inline]
    fn last_mut(&mut self) -> Option<&mut ActivePhase> {
        self.len.checked_sub(1).map(|index| &mut self.data[index])
    }
}

pub struct Metrics {
    shards: Box<[MetricsShard]>,
}

impl Metrics {
    pub fn new(shards: usize) -> Self {
        Self {
            shards: std::iter::repeat_with(MetricsShard::new).take(shards).collect(),
        }
    }

    #[inline]
    pub fn event(&self, thread: usize, event: Event) {
        self.shards[thread].event(event);
    }

    #[cfg(feature = "search-metrics-time")]
    #[inline]
    pub fn scope(&self, thread: usize, phase: Phase) -> Scope {
        let shard = &self.shards[thread] as *const MetricsShard;
        self.shards[thread].enter(phase);
        Scope { shard, phase }
    }

    pub fn reset(&self) {
        for shard in &self.shards {
            shard.reset();
        }
    }

    pub fn print_tsv(&self, nodes: u64, nps: f64) {
        self.snapshot().print_tsv(nodes, nps);
    }

    fn snapshot(&self) -> MetricsSnapshot {
        let mut events = [0u64; Event::COUNT];
        #[cfg(feature = "search-metrics-time")]
        let mut phases = [const { PhaseStats::new() }; Phase::COUNT];

        for shard in &self.shards {
            unsafe {
                for (index, value) in (*shard.events.get()).iter().enumerate() {
                    events[index] += value;
                }

                #[cfg(feature = "search-metrics-time")]
                {
                    for (index, value) in (*shard.phases.get()).iter().enumerate() {
                        phases[index].calls += value.calls;
                        phases[index].ticks += value.ticks;
                    }
                }
            }
        }

        MetricsSnapshot {
            events,
            #[cfg(feature = "search-metrics-time")]
            phases,
        }
    }
}

struct MetricsSnapshot {
    events: [u64; Event::COUNT],

    #[cfg(feature = "search-metrics-time")]
    phases: [PhaseStats; Phase::COUNT],
}

impl MetricsSnapshot {
    fn print_tsv(&self, nodes: u64, nps: f64) {
        println!("kind\tname\tvalue");
        println!("summary\tnodes\t{nodes}");
        println!("summary\tnps\t{nps:.0}");

        #[cfg(feature = "search-metrics-time")]
        {
            println!("summary\ttimer_source\t{}", timer::source());
            println!("summary\ttimer_frequency_hz\t{}", timer::frequency());
        }

        for (index, count) in self.events.into_iter().enumerate() {
            println!("event\t{}\t{count}", event_name(index));
        }

        #[cfg(feature = "search-metrics-time")]
        {
            let total_ticks = self.phases.iter().map(|phase| phase.ticks).sum::<u64>();
            for (index, stats) in self.phases.into_iter().enumerate() {
                if stats.calls > 0 || stats.ticks > 0 {
                    let name = phase_name(index);
                    let pct = if total_ticks == 0 { 0.0 } else { 100.0 * stats.ticks as f64 / total_ticks as f64 };
                    println!("phase\t{name}.calls\t{}", stats.calls);
                    println!("phase\t{name}.ticks\t{}", stats.ticks);
                    println!("phase\t{name}.pct\t{pct:.5}");
                }
            }
        }
    }
}

#[cfg(feature = "search-metrics-time")]
pub struct Scope {
    // Raw pointer avoids borrowing `ThreadData` through `td.shared.metrics` for the full scope.
    // The pointed-to shard is owned by `SharedContext` and outlives all search scopes.
    shard: *const MetricsShard,
    phase: Phase,
}

#[cfg(feature = "search-metrics-time")]
impl Drop for Scope {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            (*self.shard).exit(self.phase);
        }
    }
}

const fn event_name(index: usize) -> &'static str {
    Event::ALL[index].name()
}

const fn phase_name(index: usize) -> &'static str {
    Phase::ALL[index].name()
}

#[cfg(all(feature = "search-metrics-time", target_arch = "aarch64"))]
mod timer {
    use std::arch::asm;

    #[inline(always)]
    pub fn ticks() -> u64 {
        let value: u64;
        unsafe {
            asm!("mrs {}, cntvct_el0", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        value
    }

    pub fn frequency() -> u64 {
        let value: u64;
        unsafe {
            asm!("mrs {}, cntfrq_el0", out(reg) value, options(nomem, nostack, preserves_flags));
        }
        value
    }

    pub const fn source() -> &'static str {
        "cntvct_el0"
    }
}

#[cfg(all(feature = "search-metrics-time", not(target_arch = "aarch64")))]
mod timer {
    use std::{
        sync::OnceLock,
        time::{Duration, Instant},
    };

    static START: OnceLock<Instant> = OnceLock::new();

    #[inline]
    pub fn ticks() -> u64 {
        START.get_or_init(Instant::now).elapsed().as_nanos().min(u128::from(u64::MAX)) as u64
    }

    pub const fn frequency() -> u64 {
        Duration::from_secs(1).as_nanos() as u64
    }

    pub const fn source() -> &'static str {
        "instant"
    }
}

#[cfg(feature = "search-metrics")]
#[macro_export]
macro_rules! counter {
    ($td:expr, $event:ident $(,)?) => {
        $td.shared.metrics.event($td.id, $crate::metrics::Event::$event)
    };
    ($td:expr, $first:ident, $($event:ident),+ $(,)?) => {
        $crate::counter!($td, $first);
        $($crate::counter!($td, $event);)+
    };
}

#[cfg(not(feature = "search-metrics"))]
#[macro_export]
macro_rules! counter {
    ($td:expr, $($event:ident),+ $(,)?) => {{
        let _ = &$td;
    }};
}

#[cfg(feature = "search-metrics-time")]
#[macro_export]
macro_rules! metric_scope {
    ($td:expr, $phase:ident) => {
        $td.shared.metrics.scope($td.id, $crate::metrics::Phase::$phase)
    };
}

#[cfg(not(feature = "search-metrics-time"))]
#[macro_export]
macro_rules! metric_scope {
    ($td:expr, $phase:ident) => {
        ()
    };
}

#[cfg(feature = "search-metrics-time")]
#[macro_export]
macro_rules! finish_metric_scope {
    ($scope:expr) => {
        drop($scope)
    };
}

#[cfg(not(feature = "search-metrics-time"))]
#[macro_export]
macro_rules! finish_metric_scope {
    ($scope:expr) => {
        let _ = &$scope;
    };
}

#[cfg(feature = "search-metrics-fine-time")]
#[macro_export]
macro_rules! metric_fine_scope {
    ($td:expr, $phase:ident) => {
        $td.shared.metrics.scope($td.id, $crate::metrics::Phase::$phase)
    };
}

#[cfg(not(feature = "search-metrics-fine-time"))]
#[macro_export]
macro_rules! metric_fine_scope {
    ($td:expr, $phase:ident) => {
        ()
    };
}
