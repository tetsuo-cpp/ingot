.global _start
_start:
    and x0, x1, #0xff
    orr x0, x1, #0xff
    eor x0, x1, #0xff
    and x0, x1, x2
    orr x0, x1, x2
    eor x0, x1, x2
    tst x0, #0xff
    cmp x0, #42
    cmn x0, #1
