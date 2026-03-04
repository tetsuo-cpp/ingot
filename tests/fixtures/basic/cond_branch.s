.global _start
_start:
    cmp x0, #0
    b.eq _equal
    b.ne _not_equal
    b.lt _less
    b.ge _greater
    b.hi _high
    b.ls _low
    cbz x0, _zero
    cbnz x0, _nonzero
_equal:
_not_equal:
_less:
_greater:
_high:
_low:
_zero:
_nonzero:
    nop
