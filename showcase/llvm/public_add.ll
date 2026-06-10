; zeus.public: %0 %1
define i32 @public_add(i32 %0, i32 %1) {
entry:
  %s = add i32 %0, %1
  ret i32 %s
}
