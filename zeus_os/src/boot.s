/* boot.s - Bare-Metal Multiboot2 Entry Point for ZeusOS */

.set MAGIC, 0xE85250D6
.set ARCH, 0
.set HEADER_LENGTH, header_end - header_start
.set CHECKSUM, -(MAGIC + ARCH + HEADER_LENGTH)

.section .multiboot_header
.align 8
header_start:
    .long MAGIC
    .long ARCH
    .long HEADER_LENGTH
    .long CHECKSUM

    /* End tag */
    .short 0
    .short 0
    .long 8
header_end:

/* BSS section for stack */
.section .bss
.align 16
stack_bottom:
.skip 16384 /* 16 KiB stack */
stack_top:

.section .text
.global _start
.type _start, @function
_start:
    /* Set up the stack pointer */
    mov $stack_top, %esp

    /* Call the Zeus/C kernel entry point */
    call kernel_main

    /* Infinite loop to stop CPU if kernel returns */
1:  hlt
    jmp 1b
