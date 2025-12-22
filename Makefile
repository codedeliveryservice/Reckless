export RUSTFLAGS := -Ctarget-cpu=native

EXE := reckless

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
	PGO_MOVE := mv "target/release/reckless" "$(NAME)"
else
	PGO_MOVE := move /Y "target\release\reckless.exe" "$(NAME)"
endif

rule:
	cargo build --release
	$(PGO_MOVE)
