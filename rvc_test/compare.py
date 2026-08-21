#!/usr/bin/env python3
"""
Exhaustive RVC (compressed instruction) decoder verification.

Usage:
    1. python3 rvc_check.py --gen        # generates all_compressed.bin
    2. riscv64-linux-gnu-objdump -D -b binary -m riscv:rv64 -M no-aliases \
           all_compressed.bin > reference.dump
    3. (run your Rust dump_decode binary > my_decode.dump)
    4. python3 rvc_check.py --check      # parses both and reports mismatches
"""

import re
import struct
import subprocess
import sys
import argparse

OBJDUMP = "riscv64-linux-gnu-objdump"
ILLEGAL_MARKERS = {"unimp", ".byte", "(bad)", ".insn"}


def gen_binary(path="all_compressed.bin"):
    with open(path, "wb") as f:
        for word in range(0x10000):
            f.write(struct.pack("<H", word))
            f.write(struct.pack("<H", 0x0001))  # c.nop padding, keeps candidates 4-byte spaced
    print(f"Wrote {path} ({0x10000 * 4} bytes)")


def run_objdump(binfile="all_compressed.bin"):
    result = subprocess.run(
        [OBJDUMP, "-D", "-b", "binary", "-m", "riscv:rv64", "-M", "no-aliases", binfile],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        print("objdump failed:", result.stderr, file=sys.stderr)
        sys.exit(1)
    return result.stdout


LINE_RE = re.compile(r'^\s*([0-9a-f]+):\s+((?:[0-9a-f]{2,8}\s*)+)\t(\S+)\s*(.*)$')

def parse_objdump(text):
    """
    Returns dict: byte_addr -> (mnemonic, operands, byte_length)
    byte_length is computed from the hex-byte-group width shown by objdump.
    """
    reference = {}
    for line in text.splitlines():
        m = LINE_RE.match(line)
        if not m:
            continue
        addr = int(m.group(1), 16)
        raw_groups = m.group(2).split()
        length = sum(len(tok) // 2 for tok in raw_groups)
        mnemonic = m.group(3)
        operands = m.group(4).strip()
        reference[addr] = (mnemonic, operands, length)
    return reference


def disasm_single(word):
    """Fallback: disassemble one candidate word in total isolation."""
    data = struct.pack("<H", word) + b"\x01\x00" * 8
    tmp = "/tmp/_rvc_single.bin"
    with open(tmp, "wb") as f:
        f.write(data)
    out = subprocess.run(
        [OBJDUMP, "-D", "-b", "binary", "-m", "riscv:rv64", "-M", "no-aliases", tmp],
        capture_output=True, text=True
    ).stdout
    for line in out.splitlines():
        m = LINE_RE.match(line)
        if m and int(m.group(1), 16) == 0:
            return (m.group(3), m.group(4).strip())
    return (None, None)


def parse_mine(path="cpu_decode.dump"):
    """
    Expects lines like: '4501 I { opcode: 19, rd: 10, funct3: 0, rs1: 0, imm: 0 }'
    Returns dict: word -> debug_string
    """
    mine = {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = line.split(" ", 1)
            if len(parts) != 2:
                continue
            try:
                word = int(parts[0], 16)
            except ValueError:
                continue
            mine[word] = parts[1]
    return mine


def is_illegal_ref(mnemonic):
    return mnemonic in ILLEGAL_MARKERS


def is_unknown_mine(debug_str):
    return "Unkown" in debug_str


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gen", action="store_true", help="generate all_compressed.bin")
    ap.add_argument("--check", action="store_true", help="run the comparison")
    ap.add_argument("--bin", default="all_compressed.bin")
    ap.add_argument("--reference", default="reference.dump")
    ap.add_argument("--mine", default="cpu_decode.dump")
    ap.add_argument("--use-cached-reference", action="store_true",
                     help="don't re-run objdump, just parse an existing reference.dump")
    args = ap.parse_args()

    if args.gen:
        gen_binary(args.bin)
        return

    if not args.check:
        ap.print_help()
        return

    if args.use_cached_reference:
        with open(args.reference) as f:
            text = f.read()
    else:
        text = run_objdump(args.bin)
        with open(args.reference, "w") as f:
            f.write(text)

    reference_by_addr = parse_objdump(text)
    mine = parse_mine(args.mine)

    # Build word -> reference entry, using expected 4-byte stride,
    # falling back to per-word re-disassembly when the bulk pass
    # swallowed that address into a preceding multi-byte grouping.
    swallowed = []
    reference_by_word = {}

    for word in range(0x10000):
        if word & 0b11 == 0b11:
            continue  # not a compressed encoding
        expected_addr = word * 2  # each candidate slot is 2 (word) + 2 (padding) = 4 bytes,
                                   # but addresses in the file are byte offsets = index*4;
                                   # word index in our loop != file index, so recompute below.

    # Correct addressing: candidate #k (k = 0..65535, but we only care about non-0b11 words)
    # sits at byte offset k*4 in the file, regardless of the word's numeric value,
    # since we wrote ALL 65536 words sequentially including the 0b11 ones.
    for k, word in enumerate(range(0x10000)):
        if word & 0b11 == 0b11:
            continue
        byte_addr = k * 4
        entry = reference_by_addr.get(byte_addr)
        if entry is None:
            swallowed.append(word)
        else:
            reference_by_word[word] = entry

    print(f"{len(swallowed)} candidate words were swallowed by multi-byte groupings; "
          f"re-disassembling individually...")
    for word in swallowed:
        mnem, ops = disasm_single(word)
        if mnem is None:
            mnem = "unimp"  # couldn't resolve at all, treat as illegal
        reference_by_word[word] = (mnem, ops or "", 2)

    # Now compare
    total = 0
    mismatches = []
    missing_from_mine = []

    for word, (mnem, ops, length) in reference_by_word.items():
        total += 1
        my_str = mine.get(word)
        if my_str is None:
            missing_from_mine.append(word)
            continue

        ref_illegal = is_illegal_ref(mnem)
        my_unknown = is_unknown_mine(my_str)

        if ref_illegal != my_unknown:
            mismatches.append((word, f"{mnem} {ops}".strip(), my_str, "legality mismatch"))
            continue

        # both agree it's legal (or both agree it's illegal) -- for legal ones,
        # you can extend this with field-level comparison once you trust the
        # legality classification; left as a manual follow-up since it needs
        # per-mnemonic field mapping (rd/rs1/rs2/imm) against your DecodedInstruction shape.

    print(f"\nChecked {total} compressed encodings.")
    print(f"{len(missing_from_mine)} words missing from your decode dump.")
    print(f"{len(mismatches)} legality mismatches (reference says illegal/legal, yours disagrees):\n")

    for word, ref_str, my_str, reason in mismatches[:100]:
        print(f"  {word:04x}: objdump='{ref_str}'  mine='{my_str}'  [{reason}]")

    if missing_from_mine:
        print(f"\nFirst few missing words: {[hex(w) for w in missing_from_mine[:20]]}")


if __name__ == "__main__":
    main()
