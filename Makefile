all: k210

k210:
	@cd kernel/boards/k210 && make all MODE=release LOG=none

.PHONY: all k210
