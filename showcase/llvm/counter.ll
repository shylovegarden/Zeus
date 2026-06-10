; zeus.public: %0
define i32 @counter(i32 %0) {
entry:
  br label %loop
loop:
  %i = phi i32 [ 0, %entry ], [ %n, %loop ]
  %n = add i32 %i, 1
  %c = icmp slt i32 %n, %0
  br i1 %c, label %loop, label %done
done:
  ret i32 %i
}
