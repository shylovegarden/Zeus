; shift a constant by a SECRET amount -> variable-time. Must NOT be PROVED-SAFE.
define i32 @sshift(i32 %s) {
entry:
  %r = shl i32 1, %s
  ret i32 %r
}
