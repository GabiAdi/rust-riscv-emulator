ARCH	= riscv64-linux-gnu
AS		= $(ARCH)-as
LD		= $(ARCH)-ld
OC		= $(ARCH)-objcopy
NM		=	$(ARCH)-nm
FLAGS	=	-march=rv64imacfdzicsr -g
FLAGSC =	-march=rv64imacfdzicsr -g

.PHONY: all clean test firmware firmware.bin

all: test.bin testc.bin firmware.bin

testc.o: assembly/test.s
	$(AS) $(FLAGSC) -o assembly/testc.o assembly/testc.s

test.o: assembly/test.s
	$(AS) $(FLAGS) -o assembly/test.o assembly/test.s

test.elf: test.o
	$(LD) -T assembly/link.ld -o assembly/test.elf assembly/test.o

testc.elf: testc.o
	$(LD) -T assembly/link.ld -o assembly/testc.elf assembly/testc.o

test.bin: test.elf
	$(OC) -O binary assembly/test.elf assembly/test.bin

testc.bin: testc.elf
	$(OC) -O binary assembly/testc.elf assembly/testc.bin

firmware:
	cargo +nightly build -p firmware --target firmware/riscv64gc.json -Zjson-target-spec -Zbuild-std=core,compiler_builtins,alloc -Zbuild-std-features=compiler-builtins-mem
	cp target/riscv64gc/debug/firmware assembly/firmware.elf

firmware.bin: firmware
	$(OC) -O binary assembly/firmware.elf assembly/firmware.bin

test:
	cargo test --bin emulator

clean:
	rm -f assembly/test.o assembly/test.elf assembly/test.bin
	rm -f assembly/testc.o assembly/testc.elf assembly/testc.bin
	rm -f assembly/firmware.elf assembly/firmware.bin
	cargo clean -p firmware
