#!/usr/bin/env python3
"""Compare two profilebench TSV outputs."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path


def read_tsv(path: Path) -> dict[tuple[str, str], str]:
    rows: dict[tuple[str, str], str] = {}
    with path.open(encoding="utf-8") as file:
        header = file.readline().rstrip("\n").split("\t")
        if header != ["kind", "name", "value"]:
            raise ValueError(f"{path}: expected profilebench TSV header")

        for line in file:
            kind, name, value = line.rstrip("\n").split("\t")
            rows[(kind, name)] = value

    return rows


def numeric(value: str) -> float | None:
    try:
        return float(value)
    except ValueError:
        return None


def print_section(title: str) -> None:
    print()
    print(title)
    print("-" * len(title))


def compare(base: dict[tuple[str, str], str], current: dict[tuple[str, str], str]) -> int:
    keys = sorted(set(base) | set(current))
    status = 0

    print_section("Summary")
    for key in keys:
        kind, name = key
        if kind != "summary":
            continue
        left = base.get(key, "missing")
        right = current.get(key, "missing")
        print_delta(name, left, right)

    print_section("Event Deltas")
    event_changes = 0
    for key in keys:
        kind, name = key
        if kind != "event":
            continue
        left = base.get(key, "0")
        right = current.get(key, "0")
        if left != right:
            event_changes += 1
            print_delta(name, left, right)

    if event_changes == 0:
        print("none")
    else:
        status = 1

    print_section("Phase Deltas")
    phase_changes = print_phase_deltas(base, current)

    if phase_changes == 0:
        print("no phase pct delta >= 0.1")

    return status


def print_phase_deltas(base: dict[tuple[str, str], str], current: dict[tuple[str, str], str]) -> int:
    names = {
        name.rsplit(".", 1)[0]
        for kind, name in set(base) | set(current)
        if kind == "phase" and "." in name
    }
    rows = []

    for name in names:
        base_pct = numeric(base.get(("phase", f"{name}.pct"), "0"))
        current_pct = numeric(current.get(("phase", f"{name}.pct"), "0"))
        if base_pct is None or current_pct is None:
            continue

        base_calls = numeric(base.get(("phase", f"{name}.calls"), "0")) or 0.0
        current_calls = numeric(current.get(("phase", f"{name}.calls"), "0")) or 0.0
        base_ticks = numeric(base.get(("phase", f"{name}.ticks"), "0")) or 0.0
        current_ticks = numeric(current.get(("phase", f"{name}.ticks"), "0")) or 0.0
        base_ticks_per_call = 0.0 if base_calls == 0 else base_ticks / base_calls
        current_ticks_per_call = 0.0 if current_calls == 0 else current_ticks / current_calls
        pct_delta = current_pct - base_pct
        ticks_per_call_delta = current_ticks_per_call - base_ticks_per_call

        if abs(pct_delta) >= 0.1 or abs(ticks_per_call_delta) >= 0.1:
            rows.append(
                (
                    abs(pct_delta),
                    name,
                    base_pct,
                    current_pct,
                    pct_delta,
                    base_calls,
                    current_calls,
                    base_ticks_per_call,
                    current_ticks_per_call,
                    ticks_per_call_delta,
                )
            )

    for _, name, base_pct, current_pct, pct_delta, base_calls, current_calls, base_tpc, current_tpc, tpc_delta in sorted(
        rows, reverse=True
    ):
        print(
            f"{name}\t"
            f"{base_pct:.5f}\t{current_pct:.5f}\t{pct_delta:+.5f} pct\t"
            f"calls {base_calls:.0f}->{current_calls:.0f}\t"
            f"ticks/call {base_tpc:.3f}->{current_tpc:.3f}\t{tpc_delta:+.3f}"
        )

    return len(rows)


def print_delta(name: str, left: str, right: str) -> None:
    left_num = numeric(left)
    right_num = numeric(right)
    if left_num is None or right_num is None:
        marker = "==" if left == right else "!="
        print(f"{name}\t{left}\t{right}\t{marker}")
        return

    delta = right_num - left_num
    pct = 0.0 if left_num == 0 else 100.0 * delta / left_num
    print(f"{name}\t{left}\t{right}\t{delta:+.0f}\t{pct:+.5f}%")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base", type=Path)
    parser.add_argument("current", type=Path)
    args = parser.parse_args()

    try:
        return compare(read_tsv(args.base), read_tsv(args.current))
    except ValueError as error:
        print(error, file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
