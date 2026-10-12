define i32 @callee(i32 %input) memory(none) speculatable {
  %value = add i32 %input, 99
  ret i32 %value
}

define i32 @caller(i32 %input) memory(none) speculatable {
  %value = call i32 @callee(i32 %input) memory(none) speculatable
  ret i32 %value
}

define i1 @unsupported(i1 %input) {
  %value = xor i1 %input, true
  ret i1 %value
}
