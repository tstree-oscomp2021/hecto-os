# Building
TARGET		:= riscv64imac-unknown-none-elf
MODE		:= release
KERNEL_ELF	:= target/$(TARGET)/$(MODE)/kernel
KERNEL_BIN	:= $(KERNEL_ELF).bin
TEST_IMG	:= ../res/testsuits.img

# BOARD
BOARD		?= qemu
SBI			?= rustsbi-0.2.0-alpha.3
BOOTLOADER	:= ../res/$(SBI)-$(BOARD).bin

SMP ?= 2
LOG ?= debug

# KERNEL ENTRY
ifeq ($(BOARD), qemu)
	KERNEL_ENTRY_PA := 0x80200000
else ifeq ($(BOARD), k210)
	KERNEL_ENTRY_PA := 0x80020000
endif

# Binutils
OBJDUMP := rust-objdump --arch-name=riscv64
OBJCOPY := rust-objcopy --binary-architecture=riscv64

build: $(KERNEL_BIN)

$(KERNEL_BIN): kernel
	@$(OBJCOPY) $(KERNEL_ELF) --strip-all -O binary $@

kernel:
	@echo Platform: $(BOARD)
	@cp src/linker-$(BOARD).ld src/linker.ld
	@LOG=$(LOG) cargo build --release --features "board_$(BOARD)"
	@rm src/linker.ld

clean:
	@cargo clean

disasm: kernel
	@$(OBJDUMP) -d $(KERNEL_ELF) > target/kernel.asm
	@$(OBJDUMP) -t $(KERNEL_ELF) > target/symbol_table.asm

run: build
	@qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-smp $(SMP) \
		-bios $(BOOTLOADER) \
		-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
		-drive file=$(TEST_IMG),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

debug: build
	@tmux new-session -d \
		"qemu-system-riscv64 \
			-machine virt \
			-nographic \
			-smp $(SMP) \
			-bios $(BOOTLOADER) \
			-device loader,file=$(KERNEL_BIN),addr=$(KERNEL_ENTRY_PA) \
			-drive file=$(TEST_IMG),if=none,format=raw,id=x0 \
			-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
			-s -S" \
	&& tmux split-window -h \
		"riscv64-elf-gdb \
			-ex 'file $(KERNEL_ELF)' \
			-ex 'set arch riscv:rv64' \
			-ex 'target remote localhost:1234'" \
	&& tmux -2 attach-session -d

.PHONY: build kernel clean disasm run debug
