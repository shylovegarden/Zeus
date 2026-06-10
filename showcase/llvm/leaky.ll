define i32 @leaky(i32 %0) {
entry:
  %cmp = icmp sgt i32 %0, 0
  br i1 %cmp, label %t, label %f
t:
  ret i32 1
f:
  ret i32 0
}
