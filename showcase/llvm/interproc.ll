; outer forwards a secret to inner, which indexes a table by it (cache-timing leak).
define i32 @inner(i32 %idx, i32* %tbl) {
entry:
  %p = getelementptr i32, i32* %tbl, i32 %idx
  %v = load i32, i32* %p
  ret i32 %v
}

define i32 @outer(i32 %key, i32* %table) {
entry:
  %r = call i32 @inner(i32 %key, i32* %table)
  ret i32 %r
}
