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
	cargo pgo instrument
	cargo pgo run -- bench
	cargo pgo optimize
	$(PGO_MOVE)
