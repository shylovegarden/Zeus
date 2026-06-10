; zeus.public: %2
define i32 @insecure_cmp(ptr %0, ptr %1, i32 %2) {
entry:
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %inext, %cont ]
  %done = icmp sge i32 %i, %2
  br i1 %done, label %ok, label %body
body:
  %i64 = sext i32 %i to i64
  %pa = getelementptr inbounds i8, ptr %0, i64 %i64
  %pb = getelementptr inbounds i8, ptr %1, i64 %i64
  %ca = load i8, ptr %pa, align 1
  %cb = load i8, ptr %pb, align 1
  %ne = icmp ne i8 %ca, %cb
  br i1 %ne, label %fail, label %cont
cont:
  %inext = add i32 %i, 1
  br label %loop
fail:
  ret i32 0
ok:
  ret i32 1
}
