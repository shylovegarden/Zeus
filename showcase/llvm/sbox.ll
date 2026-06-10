define i8 @sbox_lookup(ptr %0, i8 %1) {
entry:
  %idx = zext i8 %1 to i64
  %p = getelementptr inbounds i8, ptr %0, i64 %idx
  %v = load i8, ptr %p, align 1
  ret i8 %v
}
