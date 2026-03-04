.global _start
_start:
    ldr x0, [x1]
    ldr x0, [x1, #8]
    str x0, [x1]
    str x0, [x1, #16]
    ldrb w0, [x1]
    ldrh w0, [x1]
    strb w0, [x1]
    strh w0, [x1]
    stp x0, x1, [sp, #-16]!
    ldp x0, x1, [sp], #16
