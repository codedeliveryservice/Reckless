EXE := reckless
TARGET_TUPLE := $(shell rustc --print host-tuple)

ifeq ($(OS),Windows_NT)
	EXT := .exe
	V1NAME := $(EXE)-x86_64-win-v1.exe
	V2NAME := $(EXE)-x86_64-win-v2.exe
	V3NAME := $(EXE)-x86_64-win-v3.exe
	V4NAME := $(EXE)-x86_64-win-v4.exe
	RM := rmdir /s /q
else
	EXT :=
	V1NAME := $(EXE)-x86_64-linux-v1
	V2NAME := $(EXE)-x86_64-linux-v2
	V3NAME := $(EXE)-x86_64-linux-v3
	V4NAME := $(EXE)-x86_64-linux-v4
	RM := rm -rf
endif

rule:
	cargo rustc --release -- -C target-cpu=native -C profile-generate=pgo --emit link=reckless$(EXT)
	./reckless$(EXT) bench
	llvm-profdata merge -o pgo/merged.profdata pgo
	cargo rustc --release -- -C target-cpu=native -C profile-use=pgo/merged.profdata --emit link=$(EXE)$(EXT)
	$(RM) pgo

release:
	cargo rustc --release -- -C target-cpu=x86-64 --emit link=$(V1NAME)
	cargo rustc --release -- -C target-cpu=x86-64-v2 --emit link=$(V2NAME)
	cargo rustc --release -- -C target-cpu=x86-64-v3 --emit link=$(V3NAME)
	cargo rustc --release -- -C target-cpu=x86-64-v4 --emit link=$(V4NAME)
