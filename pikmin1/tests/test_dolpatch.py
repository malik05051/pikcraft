"""Executable patches and the full (code-patched) patcher mode."""

import io
import os
import random
import struct
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from pikcraft_patcher import cli, dolpatch, gcdisc, mapbuild, modfile as mf  # noqa: E402
from tests.test_formats import make_disc  # noqa: E402
from tests.test_mapbuild import synthetic_map  # noqa: E402


def make_dol(body: bytes) -> bytes:
    """A minimal executable: one text section right after the 0x100-byte header."""
    header = bytearray(0x100)
    struct.pack_into(">I", header, 0x00, 0x100)
    struct.pack_into(">I", header, 0x90, len(body))
    return bytes(header) + body


def code_like(n: int, seed: int) -> list[int]:
    rng = random.Random(seed)
    return [rng.choice([0x48000001 | (rng.randrange(1 << 20) << 2), rng.getrandbits(32), 0x38600000 + rng.randrange(64)]) for _ in range(n)]


def modified(words: list[int]) -> list[int]:
    """New code inserted in the middle; branches after it adjusted, as a relink would do."""
    new = code_like(300, 99)
    out = words[:1000] + new + [w + (len(new) * 4 if w >> 26 == 18 else 0) for w in words[1000:]]
    return out


def pack(words: list[int]) -> bytes:
    return struct.pack(">%dI" % len(words), *[w & 0xFFFFFFFF for w in words])


class DolPatchTests(unittest.TestCase):
    def test_round_trip_and_no_verbatim_code(self):
        src_words = code_like(5000, 1)
        src, dst = pack(src_words), pack(modified(src_words))
        patch = dolpatch.make(src, dst)
        self.assertEqual(dolpatch.apply(src, patch), dst)
        self.assertLess(len(patch), len(dst) // 4)
        # The patch never carries a run of either executable as such.
        import zlib

        hdr = dolpatch.HEADER.unpack_from(patch)
        body = zlib.decompress(patch[dolpatch.HEADER.size + hdr[5] : dolpatch.HEADER.size + hdr[5] + hdr[6]])
        for blob in (src, dst):
            windows = {blob[i : i + 16] for i in range(0, len(blob) - 16, 4)}
            hits = [i for i in range(0, len(body) - 16, 4) if body[i : i + 16] in windows and any(body[i : i + 16])]
            self.assertEqual(hits, [])

    def test_rejects_other_executables(self):
        src = pack(code_like(500, 2))
        patch = dolpatch.make(src, pack(modified(code_like(500, 2))))
        with self.assertRaises(dolpatch.PatchError):
            dolpatch.apply(pack(code_like(500, 3)), patch)

    def test_full_mode_patches_code_and_maps(self):
        dol = make_dol(pack(code_like(3000, 4)))
        new_dol = make_dol(pack(modified(code_like(3000, 4))))
        files = {
            "dataDir/stages/stage1.ini": b"map_file courses/x/x.mod\n",
            "dataDir/courses/x/x.mod": synthetic_map(),
        }
        with tempfile.TemporaryDirectory() as d:
            src, dst = os.path.join(d, "in.iso"), os.path.join(d, "out.iso")
            with open(src, "wb") as f:
                f.write(make_disc(files, dol))
            with gcdisc.GameCubeDisc(src) as disc:
                self.assertEqual(disc.read_dol(), dol)
                tag = disc.version_tag
            patch_dir = os.path.join(d, "patches")
            os.mkdir(patch_dir)
            with open(os.path.join(patch_dir, f"{tag}.dolpatch"), "wb") as f:
                f.write(dolpatch.make(dol, new_dol))
            old_dir, cli.PATCH_DIR = cli.PATCH_DIR, patch_dir
            try:
                with redirect_stdout(io.StringIO()) as out:
                    self.assertEqual(cli.main([src, dst]), 0)
            finally:
                cli.PATCH_DIR = old_dir
            self.assertIn("Mode: full", out.getvalue())
            with gcdisc.GameCubeDisc(dst) as disc:
                self.assertEqual(disc.read_dol(), new_dol)
                mod = mf.ModFile.parse(disc.read("dataDir/courses/x/x.mod"))
            chunk = mod.get(mapbuild.CHUNK_BLOCKS)
            self.assertIsNotNone(chunk)
            self.assertEqual(struct.unpack_from(">I", chunk.payload)[0], mapbuild.CHUNK_MAGIC)
            self.assertEqual((8 + len(chunk.payload)) % 32, 0)
            # the original terrain is no longer drawn
            self.assertTrue(all(not j.matpolys for j in mf.decode_joints(mod.get(mf.CHUNK_JOINT).payload)))


if __name__ == "__main__":
    unittest.main()
