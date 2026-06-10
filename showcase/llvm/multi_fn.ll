; Two functions. The first is clean; the second leaks a secret via a branch.
; A single-function analyzer would only see @clean_add and miss @leak_branch.
; zeus.public: %a %b
define i32 @clean_add(i32 %a, i32 %b) {
entry:
  %s = add i32 %a, %b
  ret i32 %s
}

define i32 @leak_branch(i32 %secret) {
entry:
  %c = icmp sgt i32 %secret, 0
  br i1 %c, label %hi, label %lo
hi:
  ret i32 1
lo:
  ret i32 0
}
