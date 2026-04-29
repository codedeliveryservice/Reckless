# Testing Reckless Changes

This guide explains the layers of testing used in Reckless development,
what the `Bench:` value means, and how to set up local and OpenBench
tests without relying on assumed chess-engine workflow knowledge.

## Overview

Reckless uses four different kinds of validation:

1. Local correctness checks
2. Local benchmarking
3. CI smoke games
4. OpenBench strength testing

These layers answer different questions:

- `cargo test`, `cargo fmt`, and `cargo clippy` answer "did I break the
  build or obvious correctness?"
- `bench` answers "did I change the engine's search behavior or
  throughput on the standard bench positions?"
- the CI `fastchess` smoke test answers "does the engine stay stable in
  a minimal game run?"
- OpenBench answers "does this change improve strength?"

Do not treat a single layer as a replacement for the others.

## Local Correctness Checks

Run the same checks that CI runs:

```bash
cargo test --verbose
cargo fmt -- --check
cargo clippy -- -D warnings
```

CI also runs `cargo run --verbose -- bench`, so it is worth running
`bench` locally before opening a PR.

Relevant workflows:

- [Reckless CI](../.github/workflows/rust.yml)
- [Games](../.github/workflows/games.yml)
- [PGO](../.github/workflows/pgo.yml)

## What `bench` Does

The built-in `bench` command searches a fixed set of positions from
[`src/tools/bench.rs`](../src/tools/bench.rs)
and prints:

```text
Bench: <nodes> nodes <nps> nps
```

The important value for commit messages and OpenBench is the first
number:

- `Bench: <nodes>`

That number is the total number of nodes searched over the built-in
bench suite at the configured depth. In practice, contributors use it
as a compact fingerprint for the engine's current search behavior.

The second number:

- `<nps>`

is still useful, but it is not the canonical `Bench:` value used in
commit messages or OpenBench forms.

### Default Bench Settings

From [`src/tools/bench.rs`](../src/tools/bench.rs):

- hash: `16`
- threads: `1`
- depth: `12`

So these commands are equivalent:

```bash
cargo run -- bench
./target/release/reckless bench
./target/release/reckless 'bench 16 1 12'
```

The parameter meanings are:

- first argument: transposition-table hash size in MB
- second argument: number of search threads
- third argument: search depth

For example:

```bash
./target/release/reckless 'bench 16 1 12'
```

means "run bench with `Hash=16`, `Threads=1`, `Depth=12`".

## What to Put in the Commit Message

When maintainers ask for `Bench: ...` in the commit message, they mean
the full commit message or description should contain the node count
from `bench`, for example:

```text
Bench: 3140512
```

For Reckless, OpenBench uses this to autofill the bench field for a
test.

The usual flow is:

1. make the change
2. run `bench`
3. set the commit message to `Bench: <nodes>`
4. push your branch
5. submit OpenBench tests
6. open the PR once the test passes, or update an already-open PR with
   the result

If your change is intended to be non-functional, the bench node count
should usually stay the same. If it changes, treat that as a sign that
the patch changed engine behavior, even if the edit looked like a
micro-optimization.

## Architecture Caveat

Bench values are not always identical across architectures. In
practice, Apple Silicon and x86 can disagree on the `Bench:` node
count, likely because of architecture-specific NNUE inference details.

If your local `Bench:` value does not match what other contributors
expect:

1. run `bench` on `main`
2. run `bench` on your branch
3. ask in Discord or check recent Reckless OpenBench tests before
   submitting

Do not assume your local Apple Silicon number is the number the
Reckless OpenBench instance expects.

## CI Smoke Games

The repo's `Games` workflow uses `fastchess` as a minimal stability
smoke test, not as final Elo proof. It checks for:

- `illegal move`
- `disconnect`
- `stall`

CI pins a specific `fastchess` revision in the
[`Games` workflow](../.github/workflows/games.yml) to keep smoke-test
infrastructure reproducible.

Contributors do not generally rely on manual local `fastchess` runs as a
normal part of the Reckless workflow. In practice, the common path is:

1. local correctness checks
2. `bench`
3. OpenBench

If you want a personal sanity check, a local `fastchess` run is fine,
but treat it as optional and low-signal compared with OpenBench.

## PGO Testing

PGO stands for profile-guided optimization. Reckless uses it in CI and
in release workflows:

```bash
cargo pgo instrument
cargo pgo run -- bench
cargo pgo optimize
```

That process:

1. builds an instrumented binary
2. runs `bench` to collect profile data
3. rebuilds using the recorded profile

Small hot-path changes can disappear or reverse under PGO, so do not
rely only on plain release builds for performance claims.

If you want the exact repo-style optimized build:

```bash
make pgo
```

## Project Style Note

Reckless is a performance-focused chess engine. It does not strictly
follow conservative Rust style guidelines in the way a general-purpose
library might.

In practice, that means:

- low-level and performance-oriented code is normal here
- `unsafe` or guideline-breaking patterns are not automatically a
  problem
- the important question is whether the code is correct, measured, and
  justified for the engine

When reviewing or proposing changes, optimize for correctness,
performance evidence, and consistency with the existing codebase rather
than generic Rust style advice alone.

## OpenBench Basics

OpenBench is the main strength-testing framework for Reckless. The
upstream project describes it as a distributed framework for running
fixed-game and SPRT engine tests:

- <https://github.com/AndyGrant/OpenBench>

Reckless uses its own OpenBench instance:

- <https://recklesschess.space/>

### Important OpenBench Fields

For a normal branch-vs-main test, the key fields are:

- `Dev Source`: the repository that contains your test branch
- `Dev Sha`: the commit you want to test
- `Dev Branch`: your test branch
- `Dev Bench`: the `Bench:` node count for your dev build
- `Base Source`: the repository that contains the base branch
- `Base Sha`: the commit you want as the baseline
- `Base Branch`: usually `main`
- `Base Bench`: the `Bench:` node count for the base build
- `Dev Options` and `Base Options`: UCI options passed to the engine
  during games

### Which Repository to Use

If your development branch only exists in your fork, use your fork as
the source repository for both sides of the test.

Example:

- `Dev Source`: `https://github.com/<you>/Reckless`
- `Dev Branch`: `your-branch`
- `Base Source`: `https://github.com/<you>/Reckless`
- `Base Branch`: `main`

This works as long as your fork's `main` matches upstream `main`.

Using the upstream repo for the base side and your fork for the dev side
can be confusing if the instance expects both refs to come from the same
source repository. If in doubt, copy a recent working Reckless test and
only change the branch, SHA, and bench fields.

### What the Bench Fields Mean in OpenBench

The `Dev Bench` and `Base Bench` fields should contain the bench node
counts, not the NPS.

Example:

- correct: `3140512`
- wrong: `1133878`

### What the Engine Options Mean

OpenBench options such as:

```text
Threads=1 Hash=16 Minimal=true MoveOverhead=0
```

map to normal UCI engine options:

- `Threads=1`: use one search thread
- `Hash=16`: use a 16 MB transposition table
- `Minimal=true`: reduce UCI output noise
- `MoveOverhead=0`: reserve zero milliseconds per move for
  GUI/network overhead

This `Hash=16` is the same concept as the first argument to the local
`bench` command.

### A Good Reckless Example

This is a representative Reckless OpenBench test layout:

- dev and base both use your fork as `Source`
- dev branch points at your testing bookmark or branch
- base branch points at `main`
- both sides use the same network
- both sides use `Threads=1 Hash=16 Minimal=true MoveOverhead=0`

At the time this guide was written, a working example looked like:

```text
Dev Source  https://github.com/joshka/Reckless
Dev Branch  joshka/optimize-quiet-move-scoring
Dev Bench   2786596
Base Source https://github.com/joshka/Reckless
Base Branch main
Base Bench  2786596
Dev/Base Options Threads=1 Hash=16 Minimal=true MoveOverhead=0
```

Treat that as a template for field placement, not as a permanent
universal config. Copy a recent passing Reckless test when possible.

### Approval and Pending Tests

Some OpenBench instances auto-approve tests. Reckless does not appear to
do that for every registered user.

If a test lands in a pending state, that usually means the instance
requires an approver to accept it before workers will run it.

## Recommended Reckless Workflow

For a normal search or evaluation patch:

1. make the change
2. run `cargo test --verbose`
3. run `cargo fmt -- --check`
4. run `cargo clippy -- -D warnings`
5. run `bench`
6. set the commit message to `Bench: <nodes>`
7. push the branch to your fork
8. create an OpenBench test using your fork for both `Dev Source` and
   `Base Source`
9. open the PR after the test passes, or update an existing PR with the
   result

This ordering is intentional. In Reckless development, contributors
often run OpenBench first and only open the PR after the test looks
good.

If the change is specifically about performance:

1. compare release builds locally
2. compare PGO builds locally
3. only then rely on OpenBench to answer the Elo question

## When to Ask for Help

Ask in Discord before spending a lot of worker time if:

- your local `Bench:` value differs from what maintainers expect
- OpenBench cannot find your branch or SHA
- you are not sure whether `Base Source` should point at upstream or
  your fork
- you see a pending test and do not know whether it needs approval
- your patch changes `Bench:` when you thought it was non-functional
