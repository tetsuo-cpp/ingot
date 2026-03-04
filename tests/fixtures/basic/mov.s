.global _start
_start:
    mov x0, #0
    mov x0, #42
    mov x0, x1
    mov w0, w1
    movz x0, #0x1234
    movz x0, #0xABCD, lsl #16
    movn x0, #0
    movk x0, #0x5678, lsl #32
