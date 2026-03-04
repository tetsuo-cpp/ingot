.global _main
_main:
    stp x29, x30, [sp, #-16]!
    mov x0, #0
    ldp x29, x30, [sp], #16
    mov x16, #1
    svc #0x80
