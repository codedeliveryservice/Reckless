#!/usr/bin/env python3
"""Generate copy-paste-ready OpenBench SPSA inputs from parameters.rs."""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Sequence


INTEGER_TYPES = {
    "i8",
    "i16",
    "i32",
    "i64",
    "i128",
    "isize",
    "u8",
    "u16",
    "u32",
    "u64",
    "u128",
    "usize",
}
FLOAT_TYPES = {"f32", "f64"}
UNSIGNED_TYPES = {rust_type for rust_type in INTEGER_TYPES if rust_type.startswith("u")}

BASE_R_END = Decimal("0.002")
MIN_INTEGER_C = Decimal("0.5")

# These values are used as divisors and must not reach zero. The generated
# ranges remain centred on the current value while excluding both endpoints
# from the legacy [0, 2 * current] range.
POSITIVE_DIVISORS = {
    "eval3",
    "delta2",
    "optimism2",
    "probcut3",
    "qs1",
    "qs5",
    "corrhist1",
    "mp16",
    "history1",
    "history2",
    "history3",
    "history4",
    "history5",
    "history6",
}

# A few parameters have relationships that independent [0, 2 * current]
# ranges would violate. Keep these explicit so generated corner values cannot
# trigger invalid clamp bounds, a non-positive aspiration window, or i16
# history overflow. Lerp factors are bounded to interpolation rather than
# extrapolation.
BOUND_OVERRIDES = {
    "delta1": (Decimal("15"), Decimal("31")),
    "delta5": (Decimal("0"), Decimal("14")),
    "tm6": (Decimal("0.38055"), Decimal("1.06225")),
    "tm7": (Decimal("1.06225"), Decimal("1.74395")),
    "history5": (Decimal("69"), Decimal("32767")),
    "lerp1": (Decimal("0"), Decimal("1")),
    "lerp2": (Decimal("0"), Decimal("1")),
    "lerp3": (Decimal("0"), Decimal("1")),
    "lerp5": (Decimal("0"), Decimal("1")),
    "lerp6": (Decimal("0"), Decimal("1")),
}

DEFINE_START_RE = re.compile(r"^\s*define!\s*\(\s*(?://.*)?$")
DEFINE_END_RE = re.compile(r"^\s*\);\s*(?://.*)?$")
DECLARATION_RE = re.compile(
    r"^\s*(?P<rust_type>[A-Za-z_][A-Za-z0-9_]*)\s+"
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*"
    r"(?P<default>[^;]+?)\s*;\s*(?://.*)?$"
)


@dataclass(frozen=True)
class Parameter:
    rust_type: str
    name: str
    default: Decimal
    line_number: int

    @property
    def openbench_type(self) -> str:
        return "float" if self.rust_type in FLOAT_TYPES else "int"


def parse_number(literal: str, rust_type: str, path: Path, line_number: int) -> Decimal:
    value = literal.strip()
    if value.endswith(rust_type):
        value = value[: -len(rust_type)].rstrip("_")
    value = value.replace("_", "")

    try:
        number = Decimal(value)
    except InvalidOperation as error:
        raise ValueError(f"{path}:{line_number}: {literal!r} is not a numeric literal") from error

    if not number.is_finite():
        raise ValueError(f"{path}:{line_number}: {literal!r} is not finite")
    if rust_type in INTEGER_TYPES and number != number.to_integral_value():
        raise ValueError(f"{path}:{line_number}: {literal!r} is not an integer")
    if rust_type in UNSIGNED_TYPES and number < 0:
        raise ValueError(f"{path}:{line_number}: unsigned parameter cannot be negative")

    return number


def parse_parameters(path: Path) -> list[Parameter]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        raise ValueError(f"cannot read {path}: {error.strerror}") from error

    parameters: list[Parameter] = []
    names: set[str] = set()
    in_block = False
    blocks = 0

    for line_number, line in enumerate(lines, start=1):
        if DEFINE_START_RE.fullmatch(line):
            if in_block:
                raise ValueError(f"{path}:{line_number}: nested define! invocation")
            in_block = True
            blocks += 1
            continue

        if not in_block:
            continue

        if DEFINE_END_RE.fullmatch(line):
            in_block = False
            continue

        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue

        match = DECLARATION_RE.fullmatch(line)
        if match is None:
            raise ValueError(f"{path}:{line_number}: malformed parameter declaration: {stripped}")

        rust_type = match.group("rust_type")
        name = match.group("name")
        if rust_type not in INTEGER_TYPES | FLOAT_TYPES:
            raise ValueError(f"{path}:{line_number}: unsupported parameter type {rust_type!r}")
        if name in names:
            raise ValueError(f"{path}:{line_number}: duplicate parameter {name!r}")

        default = parse_number(match.group("default"), rust_type, path, line_number)
        parameters.append(Parameter(rust_type, name, default, line_number))
        names.add(name)

    if in_block:
        raise ValueError(f"{path}: unterminated define! invocation")
    if blocks != 1:
        raise ValueError(f"{path}: expected exactly one define! invocation, found {blocks}")
    if not parameters:
        raise ValueError(f"{path}: define! invocation contains no parameters")

    return parameters


def parameter_bounds(parameter: Parameter) -> tuple[Decimal, Decimal]:
    if parameter.name in BOUND_OVERRIDES:
        minimum, maximum = BOUND_OVERRIDES[parameter.name]
    elif parameter.name in POSITIVE_DIVISORS:
        minimum = Decimal(1)
        maximum = parameter.default * 2 - 1
    else:
        doubled = parameter.default * 2
        minimum = min(Decimal(0), doubled)
        maximum = max(Decimal(0), doubled)

    if not minimum <= parameter.default <= maximum:
        raise ValueError(
            f"parameter {parameter.name!r} default {parameter.default} is outside "
            f"generated bounds [{minimum}, {maximum}]"
        )
    if minimum == maximum:
        raise ValueError(f"parameter {parameter.name!r} has a zero-width tuning range")
    if parameter.rust_type in UNSIGNED_TYPES and minimum < 0:
        raise ValueError(f"parameter {parameter.name!r} has a negative unsigned bound")

    return minimum, maximum


def format_decimal(value: Decimal) -> str:
    result = format(value, "f")
    if "." in result:
        result = result.rstrip("0").rstrip(".")
    return "0" if result in {"", "-0"} else result


def openbench_row(parameter: Parameter) -> str:
    minimum, maximum = parameter_bounds(parameter)
    c_end = (maximum - minimum) / 20
    if c_end <= 0:
        raise ValueError(f"parameter {parameter.name!r} has a non-positive C_end")

    r_end = BASE_R_END
    if parameter.openbench_type == "int" and c_end < MIN_INTEGER_C:
        # OpenBench floors integer perturbations to 0.5. Compensate R_end for
        # deliberately smaller C values, retaining the legacy generator policy.
        r_end *= MIN_INTEGER_C / c_end

    fields = (
        parameter.name,
        parameter.openbench_type,
        format_decimal(parameter.default),
        format_decimal(minimum),
        format_decimal(maximum),
        format_decimal(c_end),
        format_decimal(r_end),
    )
    return ", ".join(fields)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate seven-field OpenBench SPSA input rows from Reckless parameters.rs."
    )
    parser.add_argument(
        "parameters",
        nargs="?",
        type=Path,
        default=Path(__file__).resolve().parent / "src" / "parameters.rs",
        help="path to parameters.rs (default: src/parameters.rs beside this script)",
    )
    args = parser.parse_args(argv)

    try:
        rows = [openbench_row(parameter) for parameter in parse_parameters(args.parameters)]
    except ValueError as error:
        parser.error(str(error))

    # codedeliveryservice/OpenBench treats an empty row as malformed input, so
    # intentionally omit the conventional final newline.
    sys.stdout.write("\n".join(rows))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
