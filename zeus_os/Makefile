# Makefile for ZeusOS (Aether)

CC = clang
CFLAGS = -target i386-pc-none-elf -ffreestanding -O2 -Wall -Wextra -nostdlib -mno-red-zone -fno-pie
AS = clang
ASFLAGS = -target i386-pc-none-elf
LDFLAGS = -target i386-pc-none-elf -nostdlib -fuse-ld=lld -Wl,-T,linker.ld -Wl,-no-pie

OBJS = src/boot.o src/kernel.o

ZeusOS.bin: $(OBJS)
	$(CC) $(LDFLAGS) -o $@ $(OBJS)

src/boot.o: src/boot.s
	$(AS) $(ASFLAGS) -c $< -o $@

src/kernel.o: src/kernel.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f src/*.o ZeusOS.bin
