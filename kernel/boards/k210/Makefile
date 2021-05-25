TARGET		:=	riscv64imac-unknown-none-elf
BOARD		:=	k210
ARCH 		:=	riscv64

include ../Makefile.common

K210_BIN	:= $(ROOT_DIR)/k210.bin
SDCARD 		:= /dev/sdb
K210_BOOTLOADER_SIZE := 131072

# Run K210
K210-SERIALPORT	:= /dev/ttyUSB0
K210-BURNER		:= $(ROOT_DIR)/res/kflash.py

all: k210

sdcard: $(TEST_IMG)
	@echo "Are you sure write to $(SDCARD) ? [y/N] " && read ans && [ $${ans:-N} = y ]
	@sudo dd if=/dev/zero of=$(SDCARD) bs=1M count=50
	@sudo dd if=$(TEST_IMG) of=$(SDCARD)

$(K210_BIN): $(KERNEL_BIN)
	cp $(BOOTLOADER) $(K210_BIN)
	dd if=$(KERNEL_BIN) of=$(K210_BIN) bs=$(K210_BOOTLOADER_SIZE) seek=1

build: $(K210_BIN)

run: build
	python3 $(K210-BURNER) -p $(K210-SERIALPORT) -b 1500000 $(K210_BIN)
	python3 -m serial.tools.miniterm --eol LF --dtr 0 --rts 0 --filter direct $(K210-SERIALPORT) 115200

.PHONY: all sdcard $(K210_BIN) build run
