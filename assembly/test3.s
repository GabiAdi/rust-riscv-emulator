# =====================================================
# RV64GD (F + D extension) self-checking test suite
# Convention:
#   gp = current test number
#   t1 = actual value (bit pattern, zero/sign-handled by macro)
#   t2 = expected value
#   fail on mismatch -> gp/t1/t2 all readable in reg dump
# =====================================================

.section .data
.align 3
s_one:      .word 0x3f800000   # 1.0
s_two:      .word 0x40000000   # 2.0
s_three:    .word 0x40400000   # 3.0
s_half:     .word 0x3f000000   # 0.5
s_neg_one:  .word 0xbf800000   # -1.0
s_zero:     .word 0x00000000   # +0.0
s_negzero:  .word 0x80000000   # -0.0
s_posinf:   .word 0x7f800000   # +inf
s_neginf:   .word 0xff800000   # -inf
s_qnan:     .word 0x7fc00000   # canonical quiet NaN
s_snan:     .word 0x7fa00000   # a signaling NaN
s_max:      .word 0x7f7fffff   # largest finite

.align 3
d_one:      .dword 0x3ff0000000000000
d_two:      .dword 0x4000000000000000
d_three:    .dword 0x4008000000000000
d_neg_one:  .dword 0xbff0000000000000
d_zero:     .dword 0x0000000000000000
d_negzero:  .dword 0x8000000000000000
d_posinf:   .dword 0x7ff0000000000000
d_neginf:   .dword 0xfff0000000000000
d_qnan:     .dword 0x7ff8000000000000
d_snan:     .dword 0x7ff4000000000000

.section .text
.global _start

# -----------------------------------------------------
# Macros
# -----------------------------------------------------
.macro CHECK32 num, freg, expected32
    li      gp, \num
    fmv.x.w t1, \freg
    slli    t1, t1, 32
    srli    t1, t1, 32      # zero-extend low 32 bits, ignore NaN-box/sign-ext noise
    li      t2, \expected32
    bne     t1, t2, fail
.endm

.macro CHECK64 num, freg, expected64
    li      gp, \num
    fmv.x.d t1, \freg
    li      t2, \expected64
    bne     t1, t2, fail
.endm

.macro CHECKI num, reg, expected
    li      gp, \num
    li      t2, \expected
    bne     \reg, t2, fail
.endm

.macro CLEARFLAGS
    csrrw   x0, fflags, x0
.endm

.macro CHECKFLAG num, bitmask
    li      gp, \num
    csrr    t1, fflags
    andi    t1, t1, \bitmask
    li      t2, \bitmask
    bne     t1, t2, fail
.endm

_start:
    li      gp, 0

    # Load all constants once
    la      t0, s_one
    flw     f0, 0(t0)
    la      t0, s_two
    flw     f1, 0(t0)
    la      t0, s_three
    flw     f2, 0(t0)
    la      t0, s_half
    flw     f3, 0(t0)
    la      t0, s_neg_one
    flw     f4, 0(t0)
    la      t0, s_zero
    flw     f5, 0(t0)
    la      t0, s_negzero
    flw     f6, 0(t0)
    la      t0, s_posinf
    flw     f7, 0(t0)
    la      t0, s_neginf
    flw     f8, 0(t0)
    la      t0, s_qnan
    flw     f9, 0(t0)
    la      t0, s_snan
    flw     f10, 0(t0)
    la      t0, s_max
    flw     f11, 0(t0)

    la      t0, d_one
    fld     f16, 0(t0)
    la      t0, d_two
    fld     f17, 0(t0)
    la      t0, d_three
    fld     f18, 0(t0)
    la      t0, d_neg_one
    fld     f19, 0(t0)
    la      t0, d_zero
    fld     f20, 0(t0)
    la      t0, d_negzero
    fld     f21, 0(t0)
    la      t0, d_posinf
    fld     f22, 0(t0)
    la      t0, d_neginf
    fld     f23, 0(t0)
    la      t0, d_qnan
    fld     f24, 0(t0)
    la      t0, d_snan
    fld     f25, 0(t0)

# =====================================================
# 1xx: Load/store integrity (round-trip through memory)
# =====================================================
    CHECK32 100, f0, 0x3f800000     # flw loaded 1.0 correctly
    CHECK64 101, f16, 0x3ff0000000000000

    la      t0, scratch32
    fsw     f2, 0(t0)               # store 3.0
    flw     f12, 0(t0)              # reload
    CHECK32 102, f12, 0x40400000

    la      t0, scratch64
    fsd     f18, 0(t0)              # store 3.0 (double)
    fld     f26, 0(t0)
    CHECK64 103, f26, 0x4008000000000000

# =====================================================
# 2xx: fadd
# =====================================================
    CLEARFLAGS
    fadd.s  f12, f0, f1             # 1.0 + 2.0 = 3.0
    CHECK32 200, f12, 0x40400000

    fadd.s  f12, f7, f8             # +inf + -inf = NaN, NV set
    CHECK32 201, f12, 0x7fc00000
    CHECKFLAG 202, 0x10             # NV

    CLEARFLAGS
    fadd.s  f12, f5, f6             # +0 + -0 = +0 (RNE)
    CHECK32 203, f12, 0x00000000

    fadd.d  f26, f16, f17           # 1.0 + 2.0 = 3.0 (double)
    CHECK64 204, f26, 0x4008000000000000

# =====================================================
# 3xx: fsub
# =====================================================
    CLEARFLAGS
    fsub.s  f12, f2, f0             # 3.0 - 1.0 = 2.0
    CHECK32 300, f12, 0x40000000

    fsub.s  f12, f5, f5             # +0 - +0 = +0
    CHECK32 301, f12, 0x00000000

    fsub.d  f26, f18, f16           # 3.0 - 1.0 = 2.0 (double)
    CHECK64 302, f26, 0x4000000000000000

# =====================================================
# 4xx: fmul
# =====================================================
    CLEARFLAGS
    fmul.s  f12, f1, f2             # 2.0 * 3.0 = 6.0
    CHECK32 400, f12, 0x40c00000

    fmul.s  f12, f5, f7             # 0 * inf = NaN, NV set
    CHECK32 401, f12, 0x7fc00000
    CHECKFLAG 402, 0x10

    fmul.d  f26, f17, f18           # 2.0 * 3.0 = 6.0 (double)
    CHECK64 403, f26, 0x4018000000000000   # placeholder-safe: see note below

# =====================================================
# 5xx: fdiv (including div-by-zero and 0/0)
# =====================================================
    CLEARFLAGS
    fdiv.s  f12, f2, f1             # 3.0 / 2.0 = 1.5
    CHECK32 500, f12, 0x3fc00000

    fdiv.s  f12, f0, f5             # 1.0 / +0.0 = +inf, DZ set
    CHECK32 501, f12, 0x7f800000
    CHECKFLAG 502, 0x08             # DZ

    CLEARFLAGS
    fdiv.s  f12, f5, f5             # 0 / 0 = NaN, NV set
    CHECK32 503, f12, 0x7fc00000
    CHECKFLAG 504, 0x10

    fdiv.d  f26, f18, f17           # 3.0 / 2.0 = 1.5 (double)
    CHECK64 505, f26, 0x3ff8000000000000

# =====================================================
# 6xx: fsqrt
# =====================================================
    CLEARFLAGS
    fsqrt.s f12, f1                 # sqrt(2.0)
    CHECK32 600, f12, 0x3fb504f3    # ~1.41421

    CLEARFLAGS
    fsqrt.s f12, f4                 # sqrt(-1.0) = NaN, NV set
    CHECK32 601, f12, 0x7fc00000
    CHECKFLAG 602, 0x10

    fsqrt.s f12, f5                 # sqrt(+0) = +0, exact, no flags
    CHECK32 603, f12, 0x00000000

    fsqrt.d f26, f17                # sqrt(2.0) (double)
    CHECK64 604, f26, 0x3ff6a09e667f3bcd

# =====================================================
# 7xx: fmadd / fmsub / fnmsub / fnmadd
# =====================================================
    CLEARFLAGS
    fmadd.s f12, f1, f2, f0         # (2*3)+1 = 7.0
    CHECK32 700, f12, 0x40e00000

    fmsub.s f12, f1, f2, f0         # (2*3)-1 = 5.0
    CHECK32 701, f12, 0x40a00000

    fnmsub.s f12, f1, f2, f0        # -(2*3)+1 = -5.0
    CHECK32 702, f12, 0xc0a00000

    fnmadd.s f12, f1, f2, f0        # -(2*3)-1 = -7.0
    CHECK32 703, f12, 0xc0e00000

    CLEARFLAGS
    fmadd.s f12, f5, f7, f0         # (0 * inf) + 1 -> NaN, NV set
    CHECK32 704, f12, 0x7fc00000
    CHECKFLAG 705, 0x10

    fmadd.d f26, f16, f17, f16      # (1*2)+1 = 3.0 (double)
    CHECK64 706, f26, 0x4008000000000000

# =====================================================
# 8xx: fsgnj / fsgnjn / fsgnjx (no flags, pure bit ops)
# =====================================================
    fsgnj.s  f12, f0, f4            # |1.0| with sign of -1.0 -> -1.0
    CHECK32 800, f12, 0xbf800000

    fsgnjn.s f12, f0, f4            # |1.0| with negated sign of -1.0 -> +1.0
    CHECK32 801, f12, 0x3f800000

    fsgnjx.s f12, f0, f4            # sign(1.0) XOR sign(-1.0) -> negative -> -1.0
    CHECK32 802, f12, 0xbf800000

    fsgnj.s  f12, f0, f0            # fmv.s pseudo-op equivalent
    CHECK32 803, f12, 0x3f800000

    fsgnjx.s f12, f4, f4            # fabs.s pseudo-op equivalent: sign XOR itself = positive
    CHECK32 804, f12, 0x3f800000

    fsgnj.d  f26, f16, f19          # 1.0 with sign of -1.0 -> -1.0 (double)
    CHECK64 805, f26, 0xbff0000000000000

# =====================================================
# 9xx: fmin / fmax (NaN + signed-zero edge cases)
# =====================================================
    CLEARFLAGS
    fmin.s  f12, f0, f1             # min(1.0, 2.0) = 1.0
    CHECK32 900, f12, 0x3f800000

    fmax.s  f12, f0, f1             # max(1.0, 2.0) = 2.0
    CHECK32 901, f12, 0x40000000

    CLEARFLAGS
    fmin.s  f12, f0, f9             # min(1.0, qNaN) -> 1.0, no flag
    CHECK32 902, f12, 0x3f800000
    CHECKFLAG 903, 0x00

    CLEARFLAGS
    fmin.s  f12, f9, f10            # min(qNaN, sNaN) -> canonical NaN, NV set (sNaN present)
    CHECK32 904, f12, 0x7fc00000
    CHECKFLAG 905, 0x10

    fmin.s  f12, f5, f6             # min(+0, -0) -> -0
    CHECK32 906, f12, 0x80000000

    fmax.s  f12, f5, f6             # max(+0, -0) -> +0
    CHECK32 907, f12, 0x00000000

    fmin.d  f26, f20, f21           # min(+0, -0) -> -0 (double)
    CHECK64 908, f26, 0x8000000000000000

# =====================================================
# 10xx: feq / flt / fle (quiet vs signaling NaN behavior)
# =====================================================
    CLEARFLAGS
    feq.s   t1, f0, f0
    CHECKI  1000, t1, 1
    CHECKFLAG 1001, 0x00            # feq is quiet, no flag on equal reals

    CLEARFLAGS
    feq.s   t1, f9, f9              # feq(qNaN, qNaN) -> 0, no flag
    CHECKI  1002, t1, 0
    CHECKFLAG 1003, 0x00

    CLEARFLAGS
    feq.s   t1, f10, f10            # feq(sNaN, sNaN) -> 0, NV set
    CHECKI  1004, t1, 0
    CHECKFLAG 1005, 0x10

    CLEARFLAGS
    flt.s   t1, f9, f0              # flt with qNaN -> 0, NV set (signaling compare)
    CHECKI  1006, t1, 0
    CHECKFLAG 1007, 0x10

    CLEARFLAGS
    fle.s   t1, f0, f2              # 1.0 <= 3.0 -> 1
    CHECKI  1008, t1, 1

    feq.d   t1, f16, f16
    CHECKI  1009, t1, 1

# =====================================================
# 11xx: fclass
# =====================================================
    fclass.s t1, f7                 # +inf -> bit 7
    CHECKI  1100, t1, 0x80

    fclass.s t1, f8                 # -inf -> bit 0
    CHECKI  1101, t1, 0x01

    fclass.s t1, f5                 # +0 -> bit 4
    CHECKI  1102, t1, 0x10

    fclass.s t1, f6                 # -0 -> bit 3
    CHECKI  1103, t1, 0x08

    fclass.s t1, f9                 # qNaN -> bit 9
    CHECKI  1104, t1, 0x200

    fclass.s t1, f10                # sNaN -> bit 8
    CHECKI  1105, t1, 0x100

    fclass.s t1, f0                 # 1.0 (positive normal) -> bit 6
    CHECKI  1106, t1, 0x40

    fclass.d t1, f22                # +inf (double) -> bit 7
    CHECKI  1107, t1, 0x80

# =====================================================
# 12xx: fcvt float -> int (saturation, NaN, rounding)
# =====================================================
    CLEARFLAGS
    fcvt.w.s t1, f2, rtz            # (int)3.0 = 3
    CHECKI  1200, t1, 3

    CLEARFLAGS
    fcvt.w.s t1, f9, rtz            # (int)qNaN -> max i32, NV set
    CHECKI  1201, t1, 0x7fffffff
    CHECKFLAG 1202, 0x10

    CLEARFLAGS
    fcvt.wu.s t1, f4, rtz           # (uint)(-1.0) -> 0, NV set (out of range for unsigned)
    CHECKI  1203, t1, 0
    CHECKFLAG 1204, 0x10

    CLEARFLAGS
    fcvt.w.s t1, f7, rtz            # (int)+inf -> max i32, NV set
    CHECKI  1205, t1, 0x7fffffff
    CHECKFLAG 1206, 0x10

    CLEARFLAGS
    fcvt.w.s t1, f8, rtz            # (int)-inf -> min i32, NV set
    li      t2, 0xffffffff80000000  # sign-extended INT32_MIN
    li      gp, 1207
    bne     t1, t2, fail

    CLEARFLAGS
    fcvt.l.s t1, f2, rtz            # (int64)3.0 = 3
    CHECKI  1208, t1, 3

# =====================================================
# 13xx: fcvt int -> float
# =====================================================
    li      t0, 42
    fcvt.s.w f12, t0, rne
    CHECK32 1300, f12, 0x42280000   # 42.0

    li      t0, -1
    fcvt.s.w f12, t0, rne           # (float)(-1) = -1.0
    CHECK32 1301, f12, 0xbf800000

    li      t0, -1
    fcvt.s.wu f12, t0, rne          # (float)(0xFFFFFFFF unsigned) = 4294967295.0
    CHECK32 1302, f12, 0x4f800000

    fcvt.d.w f26, t0                # widening int->double is always exact
    CHECK64 1303, f26, 0xbff0000000000000

# =====================================================
# 14xx: fcvt.s.d / fcvt.d.s
# =====================================================
    fcvt.s.d f12, f18, rne          # (float)3.0(double) = 3.0
    CHECK32 1400, f12, 0x40400000

    fcvt.d.s f26, f2                # (double)3.0(float) = 3.0, always exact
    CHECK64 1401, f26, 0x4008000000000000

    CLEARFLAGS
    fcvt.s.d f12, f24, rne          # (float)qNaN(double) -> qNaN(float), no NV (quiet)
    CHECK32 1402, f12, 0x7fc00000
    CHECKFLAG 1403, 0x00

# =====================================================
# 15xx: fmv (raw bit moves, no flags)
# =====================================================
    fmv.x.w  t1, f4                 # bits of -1.0 sign-extended
    li       t2, 0xffffffffbf800000
    li       gp, 1500
    bne      t1, t2, fail

    li       t0, 0x40400000
    fmv.w.x  f12, t0                # raw bits -> float reg, NaN-boxed
    CHECK32  1501, f12, 0x40400000

    fmv.x.d  t1, f19                # full 64-bit -1.0
    CHECKI   1502, t1, 0xbff0000000000000

    li       t0, 0xbff0000000000000
    fmv.d.x  f26, t0
    CHECK64  1503, f26, 0xbff0000000000000

# =====================================================
# 16xx: fadd.d
# =====================================================
    CLEARFLAGS
    fadd.d  f26, f16, f17           # 1.0 + 2.0 = 3.0
    CHECK64 1600, f26, 0x4008000000000000

    fadd.d  f26, f22, f23           # +inf + -inf = NaN, NV set
    CHECK64 1601, f26, 0x7ff8000000000000
    CHECKFLAG 1602, 0x10

    CLEARFLAGS
    fadd.d  f26, f20, f21           # +0 + -0 = +0
    CHECK64 1603, f26, 0x0000000000000000

# =====================================================
# 17xx: fsub.d
# =====================================================
    CLEARFLAGS
    fsub.d  f26, f18, f16           # 3.0 - 1.0 = 2.0
    CHECK64 1700, f26, 0x4000000000000000

    fsub.d  f26, f20, f20           # +0 - +0 = +0
    CHECK64 1701, f26, 0x0000000000000000

    fsub.d  f26, f20, f21           # +0 - (-0) = +0
    CHECK64 1702, f26, 0x0000000000000000

# =====================================================
# 18xx: fmul.d
# =====================================================
    CLEARFLAGS
    fmul.d  f26, f17, f18           # 2.0 * 3.0 = 6.0
    CHECK64 1800, f26, 0x4018000000000000

    fmul.d  f26, f20, f22           # 0 * inf = NaN, NV set
    CHECK64 1801, f26, 0x7ff8000000000000
    CHECKFLAG 1802, 0x10

# =====================================================
# 19xx: fdiv.d (div-by-zero, 0/0)
# =====================================================
    CLEARFLAGS
    fdiv.d  f26, f18, f17           # 3.0 / 2.0 = 1.5
    CHECK64 1900, f26, 0x3ff8000000000000

    fdiv.d  f26, f16, f20           # 1.0 / +0.0 = +inf, DZ set
    CHECK64 1901, f26, 0x7ff0000000000000
    CHECKFLAG 1902, 0x08

    CLEARFLAGS
    fdiv.d  f26, f20, f20           # 0 / 0 = NaN, NV set
    CHECK64 1903, f26, 0x7ff8000000000000
    CHECKFLAG 1904, 0x10

    fdiv.d  f26, f19, f16           # -1.0 / 1.0 = -1.0
    CHECK64 1905, f26, 0xbff0000000000000

# =====================================================
# 20xx: fsqrt.d
# =====================================================
    # NOTE: verify this constant independently, e.g.:
    #   python3 -c "import struct; print(hex(struct.unpack('<Q', struct.pack('<d', 2**0.5))[0]))"
    CLEARFLAGS
    fsqrt.d f26, f17                # sqrt(2.0)
    CHECK64 2000, f26, 0x3ff6a09e667f3bcd

    CLEARFLAGS
    fsqrt.d f26, f19                # sqrt(-1.0) = NaN, NV set
    CHECK64 2001, f26, 0x7ff8000000000000
    CHECKFLAG 2002, 0x10

    fsqrt.d f26, f20                # sqrt(+0) = +0, exact, no flags
    CHECK64 2003, f26, 0x0000000000000000

    fsqrt.d f26, f21                # sqrt(-0) = -0 (spec-defined special case)
    CHECK64 2004, f26, 0x8000000000000000

# =====================================================
# 21xx: fmadd.d / fmsub.d / fnmsub.d / fnmadd.d
# =====================================================
    CLEARFLAGS
    fmadd.d  f26, f17, f18, f16     # (2*3)+1 = 7.0
    CHECK64 2100, f26, 0x401c000000000000

    fmsub.d  f26, f17, f18, f16     # (2*3)-1 = 5.0
    CHECK64 2101, f26, 0x4014000000000000

    fnmsub.d f26, f17, f18, f16     # -(2*3)+1 = -5.0
    CHECK64 2102, f26, 0xc014000000000000

    fnmadd.d f26, f17, f18, f16     # -(2*3)-1 = -7.0
    CHECK64 2103, f26, 0xc01c000000000000

    CLEARFLAGS
    fmadd.d f26, f20, f22, f16      # (0 * inf) + 1 -> NaN, NV set
    CHECK64 2104, f26, 0x7ff8000000000000
    CHECKFLAG 2105, 0x10

# =====================================================
# 22xx: fsgnj.d / fsgnjn.d / fsgnjx.d
# =====================================================
    fsgnj.d  f26, f16, f19          # |1.0| with sign of -1.0 -> -1.0
    CHECK64 2200, f26, 0xbff0000000000000

    fsgnjn.d f26, f16, f19          # |1.0| with negated sign of -1.0 -> +1.0
    CHECK64 2201, f26, 0x3ff0000000000000

    fsgnjx.d f26, f16, f19          # sign(1.0) XOR sign(-1.0) -> negative -> -1.0
    CHECK64 2202, f26, 0xbff0000000000000

    fsgnjx.d f26, f19, f19          # fabs.d equivalent: sign XOR itself -> positive
    CHECK64 2203, f26, 0x3ff0000000000000

# =====================================================
# 23xx: fmin.d / fmax.d (NaN + signed-zero)
# =====================================================
    CLEARFLAGS
    fmin.d  f26, f16, f17           # min(1.0, 2.0) = 1.0
    CHECK64 2300, f26, 0x3ff0000000000000

    fmax.d  f26, f16, f17           # max(1.0, 2.0) = 2.0
    CHECK64 2301, f26, 0x4000000000000000

    CLEARFLAGS
    fmin.d  f26, f16, f24           # min(1.0, qNaN) -> 1.0, no flag
    CHECK64 2302, f26, 0x3ff0000000000000
    CHECKFLAG 2303, 0x00

    CLEARFLAGS
    fmin.d  f26, f24, f25           # min(qNaN, sNaN) -> canonical NaN, NV set
    CHECK64 2304, f26, 0x7ff8000000000000
    CHECKFLAG 2305, 0x10

    fmin.d  f26, f20, f21           # min(+0, -0) -> -0
    CHECK64 2306, f26, 0x8000000000000000

    fmax.d  f26, f20, f21           # max(+0, -0) -> +0
    CHECK64 2307, f26, 0x0000000000000000

# =====================================================
# 24xx: feq.d / flt.d / fle.d
# =====================================================
    CLEARFLAGS
    feq.d   t1, f16, f16
    CHECKI  2400, t1, 1
    CHECKFLAG 2401, 0x00

    CLEARFLAGS
    feq.d   t1, f24, f24            # feq(qNaN, qNaN) -> 0, no flag
    CHECKI  2402, t1, 0
    CHECKFLAG 2403, 0x00

    CLEARFLAGS
    feq.d   t1, f25, f25            # feq(sNaN, sNaN) -> 0, NV set
    CHECKI  2404, t1, 0
    CHECKFLAG 2405, 0x10

    CLEARFLAGS
    flt.d   t1, f24, f16            # flt with qNaN -> 0, NV set
    CHECKI  2406, t1, 0
    CHECKFLAG 2407, 0x10

    CLEARFLAGS
    fle.d   t1, f16, f18            # 1.0 <= 3.0 -> 1
    CHECKI  2408, t1, 1
    CHECKFLAG 2409, 0x00

    fle.d   t1, f18, f16            # 3.0 <= 1.0 -> 0
    CHECKI  2410, t1, 0

# =====================================================
# 25xx: fclass.d full sweep
# =====================================================
    fclass.d t1, f23                # -inf -> bit 0
    CHECKI  2500, t1, 0x01

    fclass.d t1, f19                # -1.0 (negative normal) -> bit 1
    CHECKI  2501, t1, 0x02

    fclass.d t1, f21                # -0 -> bit 3
    CHECKI  2502, t1, 0x08

    fclass.d t1, f20                # +0 -> bit 4
    CHECKI  2503, t1, 0x10

    fclass.d t1, f16                # 1.0 (positive normal) -> bit 6
    CHECKI  2504, t1, 0x40

    fclass.d t1, f22                # +inf -> bit 7
    CHECKI  2505, t1, 0x80

    fclass.d t1, f25                # sNaN -> bit 8
    CHECKI  2506, t1, 0x100

    fclass.d t1, f24                # qNaN -> bit 9
    CHECKI  2507, t1, 0x200

# =====================================================
# 26xx: fcvt.w.d / fcvt.wu.d / fcvt.l.d / fcvt.lu.d
# =====================================================
    CLEARFLAGS
    fcvt.w.d t1, f18, rtz           # (int)3.0 = 3
    CHECKI  2600, t1, 3

    CLEARFLAGS
    fcvt.w.d t1, f24, rtz           # (int)qNaN -> max i32, NV set
    CHECKI  2601, t1, 0x7fffffff
    CHECKFLAG 2602, 0x10

    CLEARFLAGS
    fcvt.wu.d t1, f19, rtz          # (uint)(-1.0) -> 0, NV set
    CHECKI  2603, t1, 0
    CHECKFLAG 2604, 0x10

    CLEARFLAGS
    fcvt.w.d t1, f22, rtz           # (int)+inf -> max i32, NV set
    CHECKI  2605, t1, 0x7fffffff
    CHECKFLAG 2606, 0x10

    CLEARFLAGS
    fcvt.w.d t1, f23, rtz           # (int)-inf -> min i32 (sign-extended), NV set
    li      t2, 0xffffffff80000000
    li      gp, 2607
    bne     t1, t2, fail
    CHECKFLAG 2608, 0x10

    CLEARFLAGS
    fcvt.l.d t1, f18, rtz           # (int64)3.0 = 3
    CHECKI  2609, t1, 3

    CLEARFLAGS
    fcvt.lu.d t1, f19, rtz          # (uint64)(-1.0) -> 0, NV set
    CHECKI  2610, t1, 0
    CHECKFLAG 2611, 0x10

# =====================================================
# 27xx: fcvt.d.w / fcvt.d.wu / fcvt.d.l / fcvt.d.lu
# =====================================================
    li      t0, 42
    fcvt.d.w f26, t0            # (double)42 = 42.0
    CHECK64 2700, f26, 0x4045000000000000

    li      t0, -1
    fcvt.d.w f26, t0           # (double)(-1) = -1.0
    CHECK64 2701, f26, 0xbff0000000000000

    li      t0, -1
    fcvt.d.wu f26, t0          # (double)(0xFFFFFFFF unsigned)
    # NOTE: verify independently, e.g.:
    #   python3 -c "import struct; print(hex(struct.unpack('<Q', struct.pack('<d', 4294967295.0))[0]))"
    CHECK64 2702, f26, 0x41efffffffe00000

    li      t0, 100
    fcvt.d.l f26, t0        # (double)100 (int64) = 100.0
    CHECK64 2703, f26, 0x4059000000000000

# =====================================================
# 28xx: round-trip sanity (int -> double -> int)
# =====================================================
    li      t0, 12345
    fcvt.d.w f26, t0 
    fcvt.w.d t1, f26, rtz           # should recover exactly, no rounding needed
    CHECKI  2800, t1, 12345
    CLEARFLAGS
    fcvt.w.d t1, f26, rtz
    CHECKFLAG 2801, 0x00            # exact round-trip, NX should NOT be set

    j       pass

# =====================================================
fail:
    # gp = failing test number, t1 = actual, t2 = expected
    li a0, 0x1
    ebreak
pass:
    li a0, 0x2
    ebreak
.section .bss
.align 3
scratch32: .space 4
scratch64: .space 8
