.global _start
_start:
    b _forward
    nop
    nop
_forward:
    b _start
