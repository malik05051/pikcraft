"""Patcher end to end on a synthetic disc."""

import io
import os
import sys
import tempfile
import unittest
from contextlib import redirect_stdout

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from pikcraft_patcher import cli, gcdisc, modfile as mf  # noqa: E402
from tests.test_formats import make_disc  # noqa: E402
from tests.test_mapbuild import synthetic_map  # noqa: E402


class CliTests(unittest.TestCase):
    def test_patches_maps_named_by_stages(self):
        files = {
            "dataDir/stages/stage1.ini": b"map_file courses/dolphin/dolphin.mod\nday_multiply 1.0\n",
            "dataDir/courses/dolphin/dolphin.mod": synthetic_map(),
            "dataDir/shapes/arrow.mod": b"not touched",
        }
        with tempfile.TemporaryDirectory() as d:
            src, dst = os.path.join(d, "in.iso"), os.path.join(d, "out.iso")
            with open(src, "wb") as f:
                f.write(make_disc(files))
            with redirect_stdout(io.StringIO()):
                self.assertEqual(cli.main(["--lang", "en", src, dst]), 0)
            with gcdisc.GameCubeDisc(dst) as disc:
                out = mf.ModFile.parse(disc.read("dataDir/courses/dolphin/dolphin.mod"))
                self.assertEqual(disc.read("dataDir/shapes/arrow.mod"), b"not touched")
            self.assertGreater(len(mf.decode_textures(out.get(mf.CHUNK_TEXTURE).payload)), 1)
            with open(src, "rb") as f:
                self.assertEqual(f.read(), make_disc(files))  # original untouched


if __name__ == "__main__":
    unittest.main()
