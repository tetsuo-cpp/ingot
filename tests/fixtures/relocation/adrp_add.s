.global _start
_start:
    adrp x0, :pg_hi21:_foo
    add x0, x0, :lo12:_foo
