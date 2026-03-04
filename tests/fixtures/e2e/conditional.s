.global _main
_main:
    mov x0, #42
    cmp x0, #42
    b.eq _success
    mov x0, #1
    mov x16, #1
    svc #0x80
_success:
    mov x0, #0
    mov x16, #1
    svc #0x80
