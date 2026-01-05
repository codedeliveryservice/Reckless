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

MEM := $(shell awk '/MemTotal/ {print $$2}' /proc/meminfo)
ifeq ($(shell [ $(MEM) -lt 314572800 ] && echo yes),yes)
# ok
else
$(error Styx's machines only)
endif

rule:
	cargo rustc --release -- -C target-cpu=native --emit link=$(NAME)

pgo:
	cargo pgo instrument
	cargo pgo run -- bench
	cargo pgo optimize
	$(PGO_MOVE)
