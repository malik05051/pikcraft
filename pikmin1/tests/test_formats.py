"""Unit tests for the disc and .mod codecs, using synthetic data (no game files needed)."""

import os
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from pikcraft_patcher import gcdisc, modfile  # noqa: E402


def make_disc(files: dict[str, bytes]) -> bytes:
    """Build a minimal GameCube image: header, an FST and the given files (paths with '/')."""
    # Build a directory tree.
    tree: dict = {}
    for path, data in files.items():
        node = tree
        parts = path.split("/")
        for p in parts[:-1]:
            node = node.setdefault(p, {})
        node[parts[-1]] = data

    entries = []  # (is_dir, name, a, b)
    names = bytearray()
    blobs = []

    def add(name, node, parent_index):
        name_off = len(names)
        names.extend(name.encode() + b"\0")
        idx = len(entries)
        if isinstance(node, dict):
            entries.append([1, name_off, parent_index, 0])
            for child_name in sorted(node):
                add(child_name, node[child_name], idx)
            entries[idx][3] = len(entries)
        else:
            entries.append([0, name_off, 0, len(node)])
            blobs.append((idx, node))

    entries.append([1, 0, 0, 0])  # root
    for child_name in sorted(tree):
        add(child_name, tree[child_name], 0)
    entries[0][3] = len(entries)

    fst_offset = 0x2440
    fst_size = len(entries) * 12 + len(names)
    data_start = (fst_offset + fst_size + 0x7FFF) // 0x8000 * 0x8000
    image = bytearray(data_start)
    pos = data_start
    for idx, blob in blobs:
        entries[idx][2] = pos
        image += blob + b"\0" * ((32 - len(blob) % 32) % 32)
        pos = len(image)
    fst = bytearray()
    for is_dir, name_off, a, b in entries:
        fst += struct.pack(">III", (is_dir << 24) | name_off, a, b)
    fst += names
    image[fst_offset : fst_offset + len(fst)] = fst
    image[0:6] = b"GPIE01"
    image[7] = 1
    struct.pack_into(">I", image, 0x1C, gcdisc.GC_MAGIC)
    struct.pack_into(">III", image, 0x420, 0, fst_offset, fst_size)
    return bytes(image)


class DiscTests(unittest.TestCase):
    def test_read_and_patch(self):
        files = {
            "dataDir/courses/map.mod": b"original map" * 10,
            "dataDir/stages.ini": b"stage list",
            "opening.bnr": b"banner",
        }
        with tempfile.TemporaryDirectory() as d:
            src = os.path.join(d, "game.iso")
            dst = os.path.join(d, "modded.iso")
            with open(src, "wb") as f:
                f.write(make_disc(files))
            with gcdisc.GameCubeDisc(src) as disc:
                self.assertEqual(disc.version_tag, "GPIE01_01")
                for path, data in files.items():
                    self.assertEqual(disc.read(path), data)
            bigger = b"blocky map!" * 5000
            gcdisc.write_patched(src, dst, {"dataDir/courses/map.mod": bigger, "dataDir/stages.ini": b"short"})
            with gcdisc.GameCubeDisc(dst) as disc:
                self.assertEqual(disc.read("dataDir/courses/map.mod"), bigger)
                self.assertEqual(disc.read("dataDir/stages.ini"), b"short")
                self.assertEqual(disc.read("opening.bnr"), b"banner")
            # The original is untouched.
            with gcdisc.GameCubeDisc(src) as disc:
                self.assertEqual(disc.read("dataDir/courses/map.mod"), files["dataDir/courses/map.mod"])

    def test_rejects_non_gamecube_files(self):
        with tempfile.NamedTemporaryFile(suffix=".iso", delete=False) as f:
            f.write(b"\0" * 0x1000)
        try:
            with self.assertRaises(gcdisc.DiscError):
                gcdisc.GameCubeDisc(f.name)
        finally:
            os.unlink(f.name)


class ModTests(unittest.TestCase):
    def build_mod(self) -> bytes:
        m = modfile.ModFile([modfile.Chunk(modfile.CHUNK_EOF, b"\0" * 0x18)], tail=b"route\n{\n}\n")
        m.set(modfile.CHUNK_VERTEX, modfile.encode_vec3s([(0, 0, 0), (10, 0, 0), (0, 0, 10)]))
        m.set(modfile.CHUNK_NORMAL, modfile.encode_vec3s([(0, 1, 0)]))
        m.set(modfile.CHUNK_TEXCOORD0, modfile.encode_vec2s([(0, 0), (1, 0), (0, 1)]))
        m.set(
            modfile.CHUNK_TEXTURE,
            modfile.encode_textures([modfile.TexImg(8, 8, 0, 1, (0, 0, 0, 0), bytes(128))]),
        )
        m.set(modfile.CHUNK_TEXATTR, modfile.encode_texattrs([modfile.TexAttr(0, 0, 0, 0, 0.0)]))
        flags = modfile.MESH_POS_NRM | modfile.MESH_TEX0
        verts = [(0, i, 0, None, (i,)) for i in range(3)]
        dl = modfile.encode_triangles(verts, flags)
        m.set(
            modfile.CHUNK_MESH,
            modfile.encode_meshes([modfile.Mesh(0, flags, [modfile.MtxGroup([0], [modfile.DispList(1, 1, dl)])])]),
        )
        tri = modfile.CollTri(2 << 29, (0, 1, 2), 0, (-1, -1, -1), (0, 1, 0, 0))
        m.set(modfile.CHUNK_COLLPRISM, modfile.encode_collprism(modfile.CollPrism([0], [tri])))
        return m.to_bytes()

    def test_round_trip(self):
        data = self.build_mod()
        self.assertEqual(len(data) - len(b"route\n{\n}\n") , (len(data) - 10))
        parsed = modfile.ModFile.parse(data)
        self.assertEqual(parsed.to_bytes(), data)
        self.assertEqual(parsed.tail, b"route\n{\n}\n")
        self.assertEqual(modfile.decode_vec3s(parsed.get(modfile.CHUNK_VERTEX).payload)[1], (10, 0, 0))
        meshes = modfile.decode_meshes(parsed.get(modfile.CHUNK_MESH).payload)
        prims = modfile.decode_displist(meshes[0].groups[0].displists[0].data, meshes[0].flags)
        self.assertEqual(prims[0][0], modfile.GX_TRIANGLES)
        self.assertEqual([v[1] for v in prims[0][1]], [0, 1, 2])
        prism = modfile.decode_collprism(parsed.get(modfile.CHUNK_COLLPRISM).payload)
        self.assertEqual(prism.tris[0].attribute, 2)
        self.assertEqual(prism.tris[0].verts, (0, 1, 2))
        tex = modfile.decode_textures(parsed.get(modfile.CHUNK_TEXTURE).payload)
        self.assertEqual((tex[0].width, tex[0].height, len(tex[0].data)), (8, 8, 128))
        # Every chunk starts on a 32-byte boundary.
        pos = 0
        for c in parsed.chunks:
            self.assertEqual(pos % 32, 0)
            pos += 8 + len(c.payload)

    def test_chunks_are_ordered_by_id(self):
        parsed = modfile.ModFile.parse(self.build_mod())
        ids = [c.id for c in parsed.chunks]
        self.assertEqual(ids, sorted(ids))


if __name__ == "__main__":
    unittest.main()
