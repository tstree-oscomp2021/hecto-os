TARGET		:=	riscv64imac-unknown-none-elf
BOARD		:=	qemu-virt-rv64
KERNEL_ENTRY_PA := 0x80200000
ARCH 		:=	riscv64
SBI			:=	rustsbi-qemu

include ../Makefile.common

run: $(KERNEL_BIN)
	@$(QEMU) $(QEMU_OPTS)

debug: $(KERNEL_BIN)
	@tmux new-session -d \
		"$(QEMU) $(QEMU_OPTS) -s -S" \
	&& tmux split-window -h \
		"$(GDB) $(GDB_OPTS)" \
	&& tmux -2 attach-session -d

.PHONY: qemu debug
