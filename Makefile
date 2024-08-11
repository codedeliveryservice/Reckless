EXE    := reckless
MODEL  := v4-ae46ed54.nnue
REPO   := https://github.com/codedeliveryservice/RecklessNetworks/raw/main

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
	V1NAME := $(EXE)-x86_64-win-v1.exe
	V2NAME := $(EXE)-x86_64-win-v2.exe
	V3NAME := $(EXE)-x86_64-win-v3.exe
	V4NAME := $(EXE)-x86_64-win-v4.exe
else
	NAME := $(EXE)
	V1NAME := $(EXE)-x86_64-linux-v1
	V2NAME := $(EXE)-x86_64-linux-v2
	V3NAME := $(EXE)-x86_64-linux-v3
	V4NAME := $(EXE)-x86_64-linux-v4
endif

rule:
	cargo rustc --release -- -C target-cpu=native --emit link=$(NAME)

datagen:
	cargo rustc --release --features=datagen -- -C target-cpu=native --emit link=$(NAME)

release:
	cargo rustc --release -- -C target-cpu=x86-64 --emit link=$(V1NAME)
	cargo rustc --release -- -C target-cpu=x86-64-v2 --emit link=$(V2NAME)
	cargo rustc --release -- -C target-cpu=x86-64-v3 --emit link=$(V3NAME)
	cargo rustc --release -- -C target-cpu=x86-64-v4 --emit link=$(V4NAME)

fetch:
	@echo Downloading $(MODEL)
	@curl -sL $(REPO)/$(MODEL) -o networks/model.nnue --create-dirs
