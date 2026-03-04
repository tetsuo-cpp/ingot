.global _start
_start:
    b _target
    nop
_target:
    bl _func
    nop
_func:
    ret
