define i32 @ct_mix(i32 %0, i32 %1) {
entry:
  %a = xor i32 %0, %1
  %b = mul i32 %a, 7
  ret i32 %b
}
