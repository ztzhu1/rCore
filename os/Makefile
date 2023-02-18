ELF_FILE := target/riscv64gc-unknown-none-elf/debug/os
BIN_FILE := $(ELF_FILE).bin
FS_IMG := ../user/target/riscv64gc-unknown-none-elf/debug/fs.img

build:
	@cargo build
	@rust-objcopy --strip-all $(ELF_FILE) -O binary $(BIN_FILE)

run: build
	@qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu.bin \
    -device loader,file=$(BIN_FILE),addr=0x80200000 \
    -drive file=$(FS_IMG),if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

debug: build
	@qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu.bin \
    -device loader,file=$(BIN_FILE),addr=0x80200000 \
	-s -S

gdb:
	@riscv64-unknown-elf-gdb \
    -ex 'file $(ELF_FILE)' \
    -ex 'set arch riscv:rv64' \
    -ex 'target remote localhost:1234'

app:
	@cd ../user && make build
	@rm -rf target/riscv64gc-unknown-none-elf

APP_PATH := ../user/src/bin
APPS := $(wildcard $(APP_PATH)/*.rs)

link_app:
	@rustc build.rs && ./build
	@rm ./build

.PHONY: build run debug gdb link_app app
