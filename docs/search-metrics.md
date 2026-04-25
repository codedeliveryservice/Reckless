# Search Metrics

`profilebench` runs the normal bench corpus with named search counters and
optional phase timing. It exists for branch comparisons where `bench` preserves
node count but NPS changes.

`dbg_hit` is still useful for temporary probes after the suspect branch or
formula is known. `profilebench` provides a reusable search-phase map: TT
probes and cutoffs, pruning counts, qsearch candidates, beta cutoffs,
reductions, and timing for eval setup, move ordering, child search, qsearch,
and finalization.

The default build compiles the instrumentation out. Build the profiler explicitly:

```bash
cargo rustc --release --features search-metrics-time -- -C target-cpu=native
```

Run the command as one CLI argument when passing parameters:

```bash
target/release/reckless "profilebench 16 1 12"
```

The arguments match `bench`: hash, threads, and depth. The output is TSV with
summary rows, event rows, and timing rows. Event rows always print, including
zero counts, so two runs can be compared without normalizing missing keys.

On aarch64, timing uses the ARM generic counter (`cntvct_el0`) and reports
`cntfrq_el0` as `timer_frequency_hz`. Other targets compile through an
`Instant` fallback.

Use the profiler in this order:

1. Compare `summary.nodes`.
2. Compare `event` rows.
3. Compare coarse `phase` rows only when nodes and events match.
4. Rebuild with `search-metrics-fine-time` after a coarse phase identifies the
   suspect area.

Matching nodes and matching events mean the tested bench corpus kept the same
search shape. Different event counts indicate behavior drift. Matching event
counts with slower NPS point at execution cost, code layout, inlining, cache
behavior, or register pressure.

Compare two saved outputs with:

```bash
scripts/compare-profilebench.py base.tsv current.tsv
```

The script exits non-zero when event counts differ. Timing from different search
shapes mixes behavior changes with implementation cost.

Example comparison:

```text
Summary
-------
nodes   2722250  2722250  +0      +0.00000%
nps     1072096  1160058  +87962  +8.20468%

Event Deltas
------------
none

Phase Deltas
------------
eval_setup  +0.99721 pct  calls 1970483->1970483  ticks/call 10.245->9.745
move_loop    +0.91541 pct  calls 796153->796153    ticks/call 0.930->1.497
```

Timing scopes are exclusive. Recursive child search is charged to
`child_search` rather than to the parent `move_loop`, and nested fine scopes
pause their parent phase.

The phase map names algorithm concepts such as `eval_setup`,
`pre_move_pruning`, `move_loop`, and `qsearch_stand_pat`. Some phases name
mechanical cost centers inside those concepts: `move_picker`, `make_undo`,
`reduction`, and `tt_access`.

The timed build changes code shape and should not replace normal `bench`, speed
testing, or external profilers.
