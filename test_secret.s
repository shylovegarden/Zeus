	.section	__TEXT,__text,regular,pure_instructions
	.build_version macos, 26, 0	sdk_version 26, 2
	.globl	_zeus_tls_handshake             ; -- Begin function zeus_tls_handshake
	.p2align	2
_zeus_tls_handshake:                    ; @zeus_tls_handshake
	.cfi_startproc
; %bb.0:
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_zeus_enclave_verify_token      ; -- Begin function zeus_enclave_verify_token
	.p2align	2
_zeus_enclave_verify_token:             ; @zeus_enclave_verify_token
	.cfi_startproc
; %bb.0:
	mov	w0, #1                          ; =0x1
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	___zeus_serialize_mutation_ledger ; -- Begin function __zeus_serialize_mutation_ledger
	.p2align	2
___zeus_serialize_mutation_ledger:      ; @__zeus_serialize_mutation_ledger
	.cfi_startproc
; %bb.0:
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_zeus_runtime_dispatch          ; -- Begin function zeus_runtime_dispatch
	.p2align	2
_zeus_runtime_dispatch:                 ; @zeus_runtime_dispatch
	.cfi_startproc
; %bb.0:
	stp	x26, x25, [sp, #-80]!           ; 16-byte Folded Spill
	stp	x24, x23, [sp, #16]             ; 16-byte Folded Spill
	stp	x22, x21, [sp, #32]             ; 16-byte Folded Spill
	stp	x20, x19, [sp, #48]             ; 16-byte Folded Spill
	stp	x29, x30, [sp, #64]             ; 16-byte Folded Spill
	add	x29, sp, #64
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	.cfi_offset w21, -40
	.cfi_offset w22, -48
	.cfi_offset w23, -56
	.cfi_offset w24, -64
	.cfi_offset w25, -72
	.cfi_offset w26, -80
	mov	x21, x2
	mov	x19, x1
	mov	x20, x0
Lloh0:
	adrp	x8, __MergedGlobals@PAGE
Lloh1:
	add	x8, x8, __MergedGlobals@PAGEOFF
	mov	w23, #1                         ; =0x1
	swpalb	w23, w8, [x8]
	tbnz	w8, #0, LBB3_3
; %bb.1:
	mov	w0, #58                         ; =0x3a
	bl	_sysconf
	mov	w8, #4                          ; =0x4
	cmp	w0, #1
	csel	w8, w8, w0, lt
Lloh2:
	adrp	x24, __MergedGlobals@PAGE+12
Lloh3:
	add	x24, x24, __MergedGlobals@PAGEOFF+12
	str	w8, [x24]
	sxtw	x0, w8
	mov	w1, #104                        ; =0x68
	bl	_calloc
	mov	x25, #0                         ; =0x0
	mov	x26, #0                         ; =0x0
	stur	x0, [x24, #4]
Lloh4:
	adrp	x22, _zeus_worker_loop@PAGE
Lloh5:
	add	x22, x22, _zeus_worker_loop@PAGEOFF
LBB3_2:                                 ; =>This Inner Loop Header: Depth=1
	ldur	x8, [x24, #4]
	add	x8, x8, x25
	str	w26, [x8, #100]
	strb	w23, [x8, #96]
	stp	xzr, xzr, [x8, #8]
	str	wzr, [x8, #24]
	add	x0, x8, #32
	mov	x1, #0                          ; =0x0
	bl	_pthread_mutex_init
	ldur	x8, [x24, #4]
	add	x0, x8, x25
	add	x3, x8, x25
	mov	x1, #0                          ; =0x0
	mov	x2, x22
	bl	_pthread_create
	add	x26, x26, #1
	ldrsw	x8, [x24]
	add	x25, x25, #104
	cmp	x26, x8
	b.lt	LBB3_2
LBB3_3:
Lloh6:
	adrp	x22, __MergedGlobals@PAGE+4
Lloh7:
	add	x22, x22, __MergedGlobals@PAGEOFF+4
	ldaddal	w21, w8, [x22]
	add	x8, x22, #4
	ldaddal	w23, w8, [x8]
	ldr	w9, [x22, #8]
	sdiv	w10, w8, w9
	msub	w24, w10, w9, w8
	mov	w0, #24                         ; =0x18
	bl	_malloc
	mov	x21, x0
	stp	x20, x19, [x0]
	ldur	x8, [x22, #12]
	mov	w9, #104                        ; =0x68
	smaddl	x19, w24, w9, x8
	add	x20, x19, #8
	add	x0, x19, #32
	bl	_pthread_mutex_lock
	str	xzr, [x21, #16]
	ldr	x8, [x19, #16]
	add	x9, x8, #16
	cmp	x8, #0
	csel	x8, x20, x9, eq
	str	x21, [x8]
	str	x21, [x19, #16]
	add	x8, x19, #24
	ldaddal	w23, w8, [x8]
	add	x0, x19, #32
	ldp	x29, x30, [sp, #64]             ; 16-byte Folded Reload
	ldp	x20, x19, [sp, #48]             ; 16-byte Folded Reload
	ldp	x22, x21, [sp, #32]             ; 16-byte Folded Reload
	ldp	x24, x23, [sp, #16]             ; 16-byte Folded Reload
	ldp	x26, x25, [sp], #80             ; 16-byte Folded Reload
	b	_pthread_mutex_unlock
	.loh AdrpAdd	Lloh0, Lloh1
	.loh AdrpAdd	Lloh4, Lloh5
	.loh AdrpAdd	Lloh2, Lloh3
	.loh AdrpAdd	Lloh6, Lloh7
	.cfi_endproc
                                        ; -- End function
	.globl	_zeus_runtime_await             ; -- Begin function zeus_runtime_await
	.p2align	2
_zeus_runtime_await:                    ; @zeus_runtime_await
	.cfi_startproc
; %bb.0:
	stp	x28, x27, [sp, #-96]!           ; 16-byte Folded Spill
	stp	x26, x25, [sp, #16]             ; 16-byte Folded Spill
	stp	x24, x23, [sp, #32]             ; 16-byte Folded Spill
	stp	x22, x21, [sp, #48]             ; 16-byte Folded Spill
	stp	x20, x19, [sp, #64]             ; 16-byte Folded Spill
	stp	x29, x30, [sp, #80]             ; 16-byte Folded Spill
	add	x29, sp, #80
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	.cfi_offset w21, -40
	.cfi_offset w22, -48
	.cfi_offset w23, -56
	.cfi_offset w24, -64
	.cfi_offset w25, -72
	.cfi_offset w26, -80
	.cfi_offset w27, -88
	.cfi_offset w28, -96
Lloh8:
	adrp	x20, __MergedGlobals@PAGE+4
Lloh9:
	add	x20, x20, __MergedGlobals@PAGEOFF+4
	ldar	w8, [x20]
	cmp	w8, #1
	b.lt	LBB4_19
; %bb.1:
	adrp	x21, __MergedGlobals@PAGE+12
	adrp	x22, __MergedGlobals@PAGE+16
	mov	w23, #-1                        ; =0xffffffff
	b	LBB4_3
LBB4_2:                                 ;   in Loop: Header=BB4_3 Depth=1
	mov	w0, #10                         ; =0xa
	bl	_usleep
	ldar	w8, [x20]
	cmp	w8, #0
	b.le	LBB4_19
LBB4_3:                                 ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB4_7 Depth 2
                                        ;     Child Loop BB4_14 Depth 2
	ldr	w8, [x21, __MergedGlobals@PAGEOFF+12]
	cmp	w8, #1
	b.lt	LBB4_2
; %bb.4:                                ;   in Loop: Header=BB4_3 Depth=1
	mov	x24, #0                         ; =0x0
	mov	w25, #1                         ; =0x1
	b	LBB4_7
LBB4_5:                                 ;   in Loop: Header=BB4_7 Depth=2
	str	xzr, [x19, #16]
	add	x8, x27, x24
	add	x8, x8, #24
	ldaddal	w23, w8, [x8]
LBB4_6:                                 ;   in Loop: Header=BB4_7 Depth=2
	add	x0, x26, #32
	bl	_pthread_mutex_unlock
	ldr	w8, [x21, __MergedGlobals@PAGEOFF+12]
	sxtw	x9, w8
	add	x24, x24, #104
	cmp	x19, #0
	ccmp	x25, x9, #0, eq
	add	x25, x25, #1
	b.ge	LBB4_10
LBB4_7:                                 ;   Parent Loop BB4_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	ldr	x27, [x22, __MergedGlobals@PAGEOFF+16]
	add	x26, x27, x24
	add	x0, x26, #32
	bl	_pthread_mutex_lock
	ldr	x19, [x26, #8]
	cbz	x19, LBB4_6
; %bb.8:                                ;   in Loop: Header=BB4_7 Depth=2
	ldr	x8, [x19, #16]
	str	x8, [x26, #8]
	cbnz	x8, LBB4_5
; %bb.9:                                ;   in Loop: Header=BB4_7 Depth=2
	add	x8, x27, x24
	str	xzr, [x8, #16]
	b	LBB4_5
LBB4_10:                                ;   in Loop: Header=BB4_3 Depth=1
	cbz	x19, LBB4_12
LBB4_11:                                ;   in Loop: Header=BB4_3 Depth=1
	ldp	x8, x0, [x19]
	blr	x8
	ldaddal	w23, w8, [x20]
	mov	x0, x19
	bl	_free
	ldar	w8, [x20]
	cmp	w8, #0
	b.gt	LBB4_3
	b	LBB4_19
LBB4_12:                                ;   in Loop: Header=BB4_3 Depth=1
	cmp	w8, #1
	b.lt	LBB4_2
; %bb.13:                               ;   in Loop: Header=BB4_3 Depth=1
	mov	x24, #0                         ; =0x0
	mov	x25, #0                         ; =0x0
LBB4_14:                                ;   Parent Loop BB4_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	ldr	x27, [x22, __MergedGlobals@PAGEOFF+16]
	add	x26, x27, x24
	add	x0, x26, #32
	bl	_pthread_mutex_lock
	ldr	x19, [x26, #8]
	cbnz	x19, LBB4_16
; %bb.15:                               ;   in Loop: Header=BB4_14 Depth=2
	add	x0, x26, #32
	bl	_pthread_mutex_unlock
	ldrsw	x8, [x21, __MergedGlobals@PAGEOFF+12]
	add	x25, x25, #1
	add	x24, x24, #104
	cmp	x25, x8
	b.lt	LBB4_14
	b	LBB4_2
LBB4_16:                                ;   in Loop: Header=BB4_3 Depth=1
	ldr	x8, [x19, #16]
	str	x8, [x26, #8]
	cbnz	x8, LBB4_18
; %bb.17:                               ;   in Loop: Header=BB4_3 Depth=1
	add	x8, x27, x24
	str	xzr, [x8, #16]
LBB4_18:                                ;   in Loop: Header=BB4_3 Depth=1
	str	xzr, [x19, #16]
	add	x8, x27, x24
	add	x8, x8, #24
	ldaddal	w23, w8, [x8]
	add	x0, x26, #32
	bl	_pthread_mutex_unlock
	b	LBB4_11
LBB4_19:
	ldp	x29, x30, [sp, #80]             ; 16-byte Folded Reload
	ldp	x20, x19, [sp, #64]             ; 16-byte Folded Reload
	ldp	x22, x21, [sp, #48]             ; 16-byte Folded Reload
	ldp	x24, x23, [sp, #32]             ; 16-byte Folded Reload
	ldp	x26, x25, [sp, #16]             ; 16-byte Folded Reload
	ldp	x28, x27, [sp], #96             ; 16-byte Folded Reload
	ret
	.loh AdrpAdd	Lloh8, Lloh9
	.cfi_endproc
                                        ; -- End function
	.globl	___zeus_repack_aos_to_soa       ; -- Begin function __zeus_repack_aos_to_soa
	.p2align	2
___zeus_repack_aos_to_soa:              ; @__zeus_repack_aos_to_soa
	.cfi_startproc
; %bb.0:
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	___zeus_arena_alloc             ; -- Begin function __zeus_arena_alloc
	.p2align	2
___zeus_arena_alloc:                    ; @__zeus_arena_alloc
	.cfi_startproc
; %bb.0:
	stp	x29, x30, [sp, #-16]!           ; 16-byte Folded Spill
	mov	x29, sp
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	add	x8, x0, #7
	and	x9, x8, #0xfffffffffffffff8
	adrp	x8, __MergedGlobals@PAGE+24
	ldr	x0, [x8, __MergedGlobals@PAGEOFF+24]
	cbz	x0, LBB6_7
; %bb.1:
	ldr	x10, [x0]
	cmp	x10, x9
	b.hs	LBB6_9
; %bb.2:
	mov	x10, x0
LBB6_3:                                 ; =>This Inner Loop Header: Depth=1
	ldr	x0, [x10, #8]
	cbz	x0, LBB6_7
; %bb.4:                                ;   in Loop: Header=BB6_3 Depth=1
	ldr	x12, [x0]
	mov	x11, x10
	mov	x10, x0
	cmp	x12, x9
	b.lo	LBB6_3
; %bb.5:
	ldr	x9, [x0, #8]
	cbz	x11, LBB6_10
LBB6_6:
	str	x9, [x11, #8]
	ldp	x29, x30, [sp], #16             ; 16-byte Folded Reload
	ret
LBB6_7:
	adrp	x8, __MergedGlobals@PAGE+32
	ldr	x10, [x8, __MergedGlobals@PAGEOFF+32]
	mov	w11, #1                         ; =0x1
	movk	w11, #1024, lsl #16
	add	x9, x10, x9
	cmp	x9, x11
	b.hs	LBB6_11
; %bb.8:
Lloh10:
	adrp	x11, ___zeus_arena@PAGE
Lloh11:
	add	x11, x11, ___zeus_arena@PAGEOFF
	add	x0, x11, x10
	str	x9, [x8, __MergedGlobals@PAGEOFF+32]
	ldp	x29, x30, [sp], #16             ; 16-byte Folded Reload
	ret
LBB6_9:
	mov	x11, #0                         ; =0x0
	ldr	x9, [x0, #8]
	cbnz	x11, LBB6_6
LBB6_10:
	str	x9, [x8, __MergedGlobals@PAGEOFF+24]
	ldp	x29, x30, [sp], #16             ; 16-byte Folded Reload
	ret
LBB6_11:
Lloh12:
	adrp	x8, ___stderrp@GOTPAGE
Lloh13:
	ldr	x8, [x8, ___stderrp@GOTPAGEOFF]
Lloh14:
	ldr	x3, [x8]
Lloh15:
	adrp	x0, l_.str@PAGE
Lloh16:
	add	x0, x0, l_.str@PAGEOFF
	mov	w1, #50                         ; =0x32
	mov	w2, #1                          ; =0x1
	bl	_fwrite
	mov	w0, #1                          ; =0x1
	bl	_exit
	.loh AdrpAdd	Lloh10, Lloh11
	.loh AdrpAdd	Lloh15, Lloh16
	.loh AdrpLdrGotLdr	Lloh12, Lloh13, Lloh14
	.cfi_endproc
                                        ; -- End function
	.globl	___zeus_arena_free              ; -- Begin function __zeus_arena_free
	.p2align	2
___zeus_arena_free:                     ; @__zeus_arena_free
	.cfi_startproc
; %bb.0:
	cbz	x0, LBB7_2
; %bb.1:
	add	x8, x1, #7
	adrp	x9, __MergedGlobals@PAGE+24
	ldr	x10, [x9, __MergedGlobals@PAGEOFF+24]
	and	x8, x8, #0xfffffffffffffff8
	stp	x8, x10, [x0]
	str	x0, [x9, __MergedGlobals@PAGEOFF+24]
LBB7_2:
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	___zeus_arena_reset             ; -- Begin function __zeus_arena_reset
	.p2align	2
___zeus_arena_reset:                    ; @__zeus_arena_reset
	.cfi_startproc
; %bb.0:
Lloh17:
	adrp	x8, __MergedGlobals@PAGE+24
Lloh18:
	add	x8, x8, __MergedGlobals@PAGEOFF+24
	stp	xzr, xzr, [x8]
	ret
	.loh AdrpAdd	Lloh17, Lloh18
	.cfi_endproc
                                        ; -- End function
	.globl	_zeus_free_tensor               ; -- Begin function zeus_free_tensor
	.p2align	2
_zeus_free_tensor:                      ; @zeus_free_tensor
	.cfi_startproc
; %bb.0:
	cbz	x0, LBB9_3
; %bb.1:
	ldr	x8, [x0]
	cbz	x8, LBB9_3
; %bb.2:
	ldp	x9, x10, [x0, #8]
	mul	x9, x9, x10
	lsl	x9, x9, #3
	adrp	x10, __MergedGlobals@PAGE+24
	ldr	x11, [x10, __MergedGlobals@PAGEOFF+24]
	stp	x9, x11, [x8]
	str	x8, [x10, __MergedGlobals@PAGEOFF+24]
	str	xzr, [x0]
LBB9_3:
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	___zeus_safestate_handler       ; -- Begin function __zeus_safestate_handler
	.p2align	2
___zeus_safestate_handler:              ; @__zeus_safestate_handler
	.cfi_startproc
; %bb.0:
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_main                           ; -- Begin function main
	.p2align	2
_main:                                  ; @main
	.cfi_startproc
; %bb.0:
	mov	w0, #0                          ; =0x0
	ret
	.cfi_endproc
                                        ; -- End function
	.p2align	2                               ; -- Begin function zeus_worker_loop
_zeus_worker_loop:                      ; @zeus_worker_loop
	.cfi_startproc
; %bb.0:
	sub	sp, sp, #112
	stp	x28, x27, [sp, #16]             ; 16-byte Folded Spill
	stp	x26, x25, [sp, #32]             ; 16-byte Folded Spill
	stp	x24, x23, [sp, #48]             ; 16-byte Folded Spill
	stp	x22, x21, [sp, #64]             ; 16-byte Folded Spill
	stp	x20, x19, [sp, #80]             ; 16-byte Folded Spill
	stp	x29, x30, [sp, #96]             ; 16-byte Folded Spill
	add	x29, sp, #96
	.cfi_def_cfa w29, 16
	.cfi_offset w30, -8
	.cfi_offset w29, -16
	.cfi_offset w19, -24
	.cfi_offset w20, -32
	.cfi_offset w21, -40
	.cfi_offset w22, -48
	.cfi_offset w23, -56
	.cfi_offset w24, -64
	.cfi_offset w25, -72
	.cfi_offset w26, -80
	.cfi_offset w27, -88
	.cfi_offset w28, -96
	add	x8, x0, #96
	ldarb	w8, [x8]
	tbz	w8, #0, LBB12_17
; %bb.1:
	mov	x19, x0
	add	x20, x0, #32
	add	x8, x0, #24
	str	x8, [sp, #8]                    ; 8-byte Folded Spill
	adrp	x24, __MergedGlobals@PAGE+12
	b	LBB12_3
LBB12_2:                                ;   in Loop: Header=BB12_3 Depth=1
	mov	w0, #100                        ; =0x64
	bl	_usleep
	add	x8, x19, #96
	ldarb	w8, [x8]
	tbz	w8, #0, LBB12_17
LBB12_3:                                ; =>This Loop Header: Depth=1
                                        ;     Child Loop BB12_11 Depth 2
	mov	x0, x20
	bl	_pthread_mutex_lock
	ldr	x21, [x19, #8]
	cbz	x21, LBB12_8
; %bb.4:                                ;   in Loop: Header=BB12_3 Depth=1
	ldr	x8, [x21, #16]
	str	x8, [x19, #8]
	cbnz	x8, LBB12_6
; %bb.5:                                ;   in Loop: Header=BB12_3 Depth=1
	str	xzr, [x19, #16]
LBB12_6:                                ;   in Loop: Header=BB12_3 Depth=1
	str	xzr, [x21, #16]
	ldr	x8, [sp, #8]                    ; 8-byte Folded Reload
	mov	x22, x20
LBB12_7:                                ;   in Loop: Header=BB12_3 Depth=1
	mov	w23, #-1                        ; =0xffffffff
	ldaddal	w23, w8, [x8]
	mov	x0, x22
	bl	_pthread_mutex_unlock
	ldp	x8, x0, [x21]
	blr	x8
Lloh19:
	adrp	x8, __MergedGlobals@PAGE+4
Lloh20:
	add	x8, x8, __MergedGlobals@PAGEOFF+4
	ldaddal	w23, w8, [x8]
	mov	x0, x21
	bl	_free
	add	x8, x19, #96
	ldarb	w8, [x8]
	tbnz	w8, #0, LBB12_3
	b	LBB12_17
LBB12_8:                                ;   in Loop: Header=BB12_3 Depth=1
	mov	x0, x20
	bl	_pthread_mutex_unlock
	ldr	w8, [x24, __MergedGlobals@PAGEOFF+12]
	cmp	w8, #1
	b.lt	LBB12_2
; %bb.9:                                ;   in Loop: Header=BB12_3 Depth=1
	mov	x27, #0                         ; =0x0
	ldr	w23, [x19, #100]
	mov	w28, #16                        ; =0x10
	b	LBB12_11
LBB12_10:                               ;   in Loop: Header=BB12_11 Depth=2
	add	x27, x27, #1
	add	x28, x28, #104
	cmp	x27, w8, sxtw
	b.ge	LBB12_2
LBB12_11:                               ;   Parent Loop BB12_3 Depth=1
                                        ; =>  This Inner Loop Header: Depth=2
	cmp	x23, x27
	b.eq	LBB12_10
; %bb.12:                               ;   in Loop: Header=BB12_11 Depth=2
Lloh21:
	adrp	x8, __MergedGlobals@PAGE+16
Lloh22:
	ldr	x26, [x8, __MergedGlobals@PAGEOFF+16]
	add	x25, x26, x28
	add	x22, x25, #16
	mov	x0, x22
	bl	_pthread_mutex_lock
	ldur	x21, [x25, #-8]
	cbnz	x21, LBB12_14
; %bb.13:                               ;   in Loop: Header=BB12_11 Depth=2
	mov	x0, x22
	bl	_pthread_mutex_unlock
	ldr	w8, [x24, __MergedGlobals@PAGEOFF+12]
	b	LBB12_10
LBB12_14:                               ;   in Loop: Header=BB12_3 Depth=1
	ldr	x8, [x21, #16]
	stur	x8, [x25, #-8]
	cbnz	x8, LBB12_16
; %bb.15:                               ;   in Loop: Header=BB12_3 Depth=1
	str	xzr, [x26, x28]
LBB12_16:                               ;   in Loop: Header=BB12_3 Depth=1
	str	xzr, [x21, #16]
	add	x8, x26, x28
	add	x8, x8, #8
	b	LBB12_7
LBB12_17:
	mov	x0, #0                          ; =0x0
	ldp	x29, x30, [sp, #96]             ; 16-byte Folded Reload
	ldp	x20, x19, [sp, #80]             ; 16-byte Folded Reload
	ldp	x22, x21, [sp, #64]             ; 16-byte Folded Reload
	ldp	x24, x23, [sp, #48]             ; 16-byte Folded Reload
	ldp	x26, x25, [sp, #32]             ; 16-byte Folded Reload
	ldp	x28, x27, [sp, #16]             ; 16-byte Folded Reload
	add	sp, sp, #112
	ret
	.loh AdrpAdd	Lloh19, Lloh20
	.loh AdrpLdr	Lloh21, Lloh22
	.cfi_endproc
                                        ; -- End function
	.section	__TEXT,__cstring,cstring_literals
l_.str:                                 ; @.str
	.asciz	"[ZEUS OOM PANIC]: Static arena capacity exceeded.\n"

.zerofill __DATA,__bss,___zeus_arena,67108864,0 ; @__zeus_arena
.zerofill __DATA,__bss,__MergedGlobals,40,3 ; @_MergedGlobals
.subsections_via_symbols
