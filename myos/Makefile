elf_file := target/riscv64gc-unknown-none-elf/debug/myos
bin_file := $(elf_file).bin

run: build
	@qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu.bin \
    -device loader,file=$(bin_file),addr=0x80200000

debug: build
	@qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios ../bootloader/rustsbi-qemu.bin \
    -device loader,file=$(bin_file),addr=0x80200000 \
    -s -S

gdb:
	@riscv64-unknown-elf-gdb \
	-ex 'file $(elf_file)' \
	-ex 'set arch riscv:rv64' \
	-ex 'target remote localhost:1234'

build:
	@cargo build 
	@rust-objcopy --strip-all $(elf_file) -O binary $(bin_file)


PHONY: run debug gdb build