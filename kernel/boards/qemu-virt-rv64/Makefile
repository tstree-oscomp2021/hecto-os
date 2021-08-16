TARGET		:=	riscv64imac-unknown-none-elf
BOARD		:=	qemu-virt-rv64
KERNEL_ENTRY_PA := 0x80200000
SBI			:=	rustsbi-qemu

include ../Makefile.common

LOOP		?=	1

run: $(KERNEL_BIN)
	@for ((i = 1; i <= $(LOOP); i++)); do $(QEMU) $(QEMU_OPTS); done

debug: $(KERNEL_BIN)
	@tmux new-session -d \
		"$(QEMU) $(QEMU_OPTS) -s -S" \
	&& tmux split-window -h \
		"$(GDB) $(GDB_OPTS)" \
	&& tmux -2 attach-session -d

.PHONY: qemu debug
