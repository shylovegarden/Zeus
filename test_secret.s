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
	.globl	___zeus_sign_mutation           ; -- Begin function __zeus_sign_mutation
	.p2align	2
___zeus_sign_mutation:                  ; @__zeus_sign_mutation
	.cfi_startproc
; %bb.0:
Lloh0:
	adrp	x0, l_.str@PAGE
Lloh1:
	add	x0, x0, l_.str@PAGEOFF
	ret
	.loh AdrpAdd	Lloh0, Lloh1
	.cfi_endproc
                                        ; -- End function
	.globl	_ibv_post_send                  ; -- Begin function ibv_post_send
	.p2align	2
_ibv_post_send:                         ; @ibv_post_send
	.cfi_startproc
; %bb.0:
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_ibv_post_recv                  ; -- Begin function ibv_post_recv
	.p2align	2
_ibv_post_recv:                         ; @ibv_post_recv
	.cfi_startproc
; %bb.0:
	ret
	.cfi_endproc
                                        ; -- End function
	.globl	_zeus_free_tensor               ; -- Begin function zeus_free_tensor
	.p2align	2
_zeus_free_tensor:                      ; @zeus_free_tensor
	.cfi_startproc
; %bb.0:
	cbz	x0, LBB6_2
; %bb.1:
	str	xzr, [x0]
LBB6_2:
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
	sub	sp, sp, #16
	.cfi_def_cfa_offset 16
	strb	wzr, [sp, #8]
	strb	wzr, [sp, #9]
	strb	wzr, [sp, #10]
	strb	wzr, [sp, #11]
	strb	wzr, [sp, #12]
	strb	wzr, [sp, #13]
	strb	wzr, [sp, #14]
	add	x8, sp, #8
	mov	x9, sp
	strb	wzr, [sp, #15]
	str	x8, [sp]
	; InlineAsm Start
	; InlineAsm End
	mov	w0, #0                          ; =0x0
	add	sp, sp, #16
	ret
	.cfi_endproc
                                        ; -- End function
	.section	__TEXT,__cstring,cstring_literals
l_.str:                                 ; @.str
	.asciz	"VALID_SIG"

.subsections_via_symbols
