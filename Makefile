export RUSTFLAGS := -Ctarget-cpu=native

EXE := reckless
TARGET_TUPLE := $(shell rustc --print host-tuple)

ifdef MSYSTEM
	NAME := $(EXE).exe
	ENV = UNIX
else ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
	ENV = WINDOWS
else
	NAME := $(EXE)
	ENV := UNIX
endif

ifeq ($(ENV),UNIX)
	PGO_MOVE := mv "target/$(TARGET_TUPLE)/release/reckless" "$(NAME)"
else
	PGO_MOVE := move /Y "target\$(TARGET_TUPLE)\release\reckless.exe" "$(NAME)"
endif

rule:
	cargo rustc --release -- -C target-cpu=native --emit link=$(NAME)

x64-check:
	RUSTFLAGS="-C target-cpu=x86-64" cargo check --target x86_64-unknown-linux-gnu --no-default-features
	RUSTFLAGS="-C target-cpu=x86-64-v3" cargo check --target x86_64-unknown-linux-gnu --no-default-features
	RUSTFLAGS="-C target-cpu=x86-64-v4 -C target-feature=+gfni,+avx512bw,+avx512vl,+avx512vbmi,+avx512vbmi2,+avx512vnni,+avx512bitalg" cargo check --target x86_64-unknown-linux-gnu --no-default-features

pgo:
	cargo pgo instrument
	cargo pgo run -- bench
	cargo pgo optimize
	$(PGO_MOVE)
