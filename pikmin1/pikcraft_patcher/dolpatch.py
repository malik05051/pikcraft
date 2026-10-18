"""Binary patches for the game executable (main.dol).

A patch rebuilds the modified executable from the player's own copy, in the style of bsdiff:

- every run takes words from the original executable and adds a difference to each. Code that only
  moved keeps its instructions, and only the branch offsets and addresses inside it change, so the
  differences are mostly zero. New code is stored as its difference from the original words at the
  same place, so the patch carries no bytes of either executable as such and is useless without the
  player's own copy.

The SHA-1 of both the original and the result are checked, so a patch only ever applies to the exact
executable it was made for.
"""

from __future__ import annotations

import hashlib
import struct
import zlib

from .i18n import tr

MAGIC = b"PCDP"
VERSION = 2
HEADER = struct.Struct(">4sI20s20sIIII")  # magic, version, src sha1, dst sha1, dst size, 3 stream sizes (the last unused)
KEY = 12  # bytes hashed to find where moved code came from
MIN_RUN = 8  # words
WINDOW = 16  # words: an "add" run continues while most of the recent words match exactly
MAX_MISMATCH = 6


class PatchError(Exception):
    pass


def _words(b: bytes) -> list[int]:
    b = b + b"\0" * (-len(b) % 4)
    return list(struct.unpack(">%dI" % (len(b) // 4), b))


def _run_len(src: list[int], s: int, dst: list[int], d: int) -> int:
    """Words from (s, d) onward that belong to one moved run (mostly identical words)."""
    limit = min(len(src) - s, len(dst) - d)
    recent: list[int] = []
    misses = 0
    last_hit = -1
    for n in range(limit):
        hit = src[s + n] == dst[d + n]
        recent.append(hit)
        if not hit:
            misses += 1
        if len(recent) > WINDOW and not recent.pop(0):
            misses -= 1
        if hit:
            last_hit = n
        elif misses > MAX_MISMATCH:
            break
    return last_hit + 1


def make(src_bytes: bytes, dst_bytes: bytes) -> bytes:
    src, dst = _words(src_bytes), _words(dst_bytes)
    index: dict[tuple, int] = {}
    k = KEY // 4
    for i in range(len(src) - k):
        index.setdefault(tuple(src[i : i + k]), i)

    runs: list[list[int]] = []  # [src word, count]
    delta = 0
    pos = 0
    while pos < len(dst):
        best, best_src = 0, 0
        for cand in (pos - delta, index.get(tuple(dst[pos : pos + k]), -1)):
            if 0 <= cand < len(src):
                n = _run_len(src, cand, dst, pos)
                if n > best:
                    best, best_src = n, cand
        if best >= MIN_RUN:
            delta = pos - best_src
        else:
            # new or rewritten words: difference from the original words at the current alignment
            best, best_src = 1, min(max(pos - delta, 0), len(src) - 1)
        if runs and runs[-1][0] + runs[-1][1] == best_src:
            runs[-1][1] += best
        else:
            runs.append([best_src, best])
        pos += best

    ops = bytearray()
    diffs = bytearray()
    pos = 0
    for s, n in runs:
        ops.extend(struct.pack(">II", s, n))
        diffs.extend(struct.pack(">%dI" % n, *((dst[pos + i] - src[s + i]) & 0xFFFFFFFF for i in range(n))))
        pos += n
    inserts = b""

    streams = [zlib.compress(bytes(s), 9) for s in (ops, diffs, inserts)]
    header = HEADER.pack(
        MAGIC,
        VERSION,
        hashlib.sha1(src_bytes).digest(),
        hashlib.sha1(dst_bytes).digest(),
        len(dst_bytes),
        *(len(s) for s in streams),
    )
    return header + b"".join(streams)


def source_sha1(patch: bytes) -> str:
    return HEADER.unpack_from(patch)[2].hex()


def apply(src_bytes: bytes, patch: bytes) -> bytes:
    magic, version, src_sha, dst_sha, size, n_ops, n_diffs, n_ins = HEADER.unpack_from(patch)
    if magic != MAGIC or version != VERSION:
        raise PatchError(tr("not_a_patch"))
    if hashlib.sha1(src_bytes).digest() != src_sha:
        raise PatchError(tr("wrong_version"))
    p = HEADER.size
    ops = zlib.decompress(patch[p : p + n_ops])
    diffs = zlib.decompress(patch[p + n_ops : p + n_ops + n_diffs])
    inserts = zlib.decompress(patch[p + n_ops + n_diffs : p + n_ops + n_diffs + n_ins])

    if inserts:
        raise PatchError(tr("corrupt_patch"))
    src = _words(src_bytes)
    out: list[int] = []
    di = 0
    for i in range(0, len(ops), 8):
        s, n = struct.unpack_from(">II", ops, i)
        if s + n > len(src):
            raise PatchError(tr("corrupt_patch"))
        d = struct.unpack_from(">%dI" % n, diffs, di)
        out.extend((src[s + j] + d[j]) & 0xFFFFFFFF for j in range(n))
        di += 4 * n
    result = struct.pack(">%dI" % len(out), *out)[:size]
    if hashlib.sha1(result).digest() != dst_sha:
        raise PatchError(tr("bad_checksum"))
    return result


def main(argv: list[str] | None = None) -> int:
    """python -m pikcraft_patcher.dolpatch <original disc image> <modified main.dol> <patch out>"""
    import sys

    from . import gcdisc

    args = sys.argv[1:] if argv is None else argv
    if len(args) != 3:
        print(main.__doc__)
        return 2
    with gcdisc.GameCubeDisc(args[0]) as disc:
        src = disc.read_dol()
    with open(args[1], "rb") as f:
        dst = f.read()
    patch = make(src, dst)
    assert apply(src, patch) == dst
    with open(args[2], "wb") as f:
        f.write(patch)
    print(f"{args[2]}: {len(patch)} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
