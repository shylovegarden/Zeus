; zeus.public: %0 %1 %2
define i32 @route(i32 %0, i32 %1, i32 %2) {
entry:
  %c = icmp sgt i32 %0, 0
  br i1 %c, label %a, label %b
a:
  %x = add i32 %1, %2
  br label %end
b:
  %y = mul i32 %1, %2
  br label %end
end:
  %r = phi i32 [ %x, %a ], [ %y, %b ]
  ret i32 %r
}
