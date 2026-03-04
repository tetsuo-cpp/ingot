.global _main
_main:
    b _skip
    mov x0, #1
_skip:
    mov x0, #0
    mov x16, #1
    svc #0x80
