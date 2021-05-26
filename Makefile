all: k210

k210:
	@cd kernel/boards/k210 && make all LOG=none

.PHONY: all k210
