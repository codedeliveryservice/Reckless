export RUSTFLAGS := -Ctarget-cpu=native

EXE := reckless
TARGET_TUPLE := $(shell rustc --print host-tuple)

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
	V1NAME := $(EXE)-x86_64-win-v1.exe
	V2NAME := $(EXE)-x86_64-win-v2.exe
	V3NAME := $(EXE)-x86_64-win-v3.exe
	V4NAME := $(EXE)-x86_64-win-v4.exe
	
	ifdef MSYSTEM
		UNIX := 1
	else
		UNIX := 0
	endif
else
	NAME := $(EXE)
	V1NAME := $(EXE)-x86_64-linux-v1
	V2NAME := $(EXE)-x86_64-linux-v2
	V3NAME := $(EXE)-x86_64-linux-v3
	V4NAME := $(EXE)-x86_64-linux-v4
	UNIX := 1
endif

ifeq ($(UNIX),1)
	PGO_MOVE := mv "target/$(TARGET_TUPLE)/release/reckless" "$(NAME)"
else
	PGO_MOVE := move /Y "target\$(TARGET_TUPLE)\release\reckless.exe" "$(NAME)"
endif

rule:
	cargo pgo instrument
	cargo pgo run -- warmup
	cargo pgo optimize
	$(PGO_MOVE)

release:
	cargo rustc --release -- -C target-cpu=x86-64 --emit link=$(V1NAME)
	cargo rustc --release -- -C target-cpu=x86-64-v2 --emit link=$(V2NAME)
	cargo rustc --release -- -C target-cpu=x86-64-v3 --emit link=$(V3NAME)
	cargo rustc --release -- -C target-cpu=x86-64-v4 --emit link=$(V4NAME)
