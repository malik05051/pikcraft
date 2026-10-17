"""End-to-end map conversion on a synthetic map model."""

import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from pikcraft_patcher import mapbuild, modfile as mf  # noqa: E402
from tests.test_voxel import terrain  # noqa: E402


def pvw_material(tex_attr: int) -> bytes:
    """Smallest PVW material with one texture slot (all animation tracks empty)."""
    b = struct.pack(">IIII", mf.MATFLAG_PVW, 0, 0xFFFFFFFF, 0)
    b += bytes(12) + struct.pack(">II", 0, 0)  # polygon colour + anim3 + anim1
    b += bytes(8) + bytes(16)  # lighting, pe
    b += struct.pack(">I", 0) + bytes(12) + struct.pack(">I", 0)  # tex info, no texgens
    b += struct.pack(">I", 1)  # one texdata
    b += struct.pack(">I", tex_attr) + bytes(2 + 2 + 4 + 4 + 4 + 4 + 28) + struct.pack(">III", 0, 0, 0)
    return b


def synthetic_map(ccw: bool = False) -> bytes:
    verts, tris = terrain()
    m = mf.ModFile([mf.Chunk(mf.CHUNK_EOF, b"\0" * 0x18)], tail=b"route\n{\n}\n")
    m.set(mf.CHUNK_VERTEX, mf.encode_vec3s(verts))
    m.set(mf.CHUNK_NORMAL, mf.encode_vec3s([(0, 1, 0)]))
    m.set(mf.CHUNK_TEXCOORD0, mf.encode_vec2s([(0, 0)]))
    m.set(mf.CHUNK_TEXTURE, mf.encode_textures([mf.TexImg(8, 8, 0, 1, (0, 0, 0, 0), bytes(128))]))
    m.set(mf.CHUNK_TEXATTR, mf.encode_texattrs([mf.TexAttr(0, 0, 0, 0, 0.0)]))
    mats = mf.MaterialChunk([], [])
    raw = pvw_material(0)
    mats.materials.append(mf.Material(raw, mf.MATFLAG_PVW, 0, [len(raw) - 4 * 3 - 56]))
    m.set(mf.CHUNK_MATERIAL, mf.encode_materials(mats))
    flags = mf.MESH_POS_NRM | mf.MESH_TEX0
    dl_verts = []
    for a, b, c, _ in tris:
        order = (a, b, c) if ccw else (a, c, b)  # terrain() triangles are counter-clockwise from above
        dl_verts += [(0, i, 0, None, (0,)) for i in order]
    dl = mf.encode_triangles(dl_verts, flags)
    m.set(mf.CHUNK_MESH, mf.encode_meshes([mf.Mesh(0, flags, [mf.MtxGroup([0], [mf.DispList(1, len(tris), dl)])])]))
    joint = mf.Joint(-1, 0, (0, 0, 0), (1, 1, 1), 0.0, (1, 1, 1), (0, 0, 0), (0, 0, 0), [(0, 0)])
    m.set(mf.CHUNK_JOINT, mf.encode_joints([joint]))
    ctris = [mf.CollTri(attr << 29, (a, b, c), 0, (-1, -1, -1), (0, 1, 0, 0)) for a, b, c, attr in tris]
    m.set(mf.CHUNK_COLLPRISM, mf.encode_collprism(mf.CollPrism([0], ctris)))
    return m.to_bytes()


class MapBuildTests(unittest.TestCase):
    def test_material_offsets_match_decoder(self):
        chunk = mf.decode_materials(mf.ModFile.parse(synthetic_map()).get(mf.CHUNK_MATERIAL).payload)
        self.assertEqual(chunk.materials[0].tex_attrs(), [0])

    def test_build_replaces_terrain_and_keeps_gameplay(self):
        src = synthetic_map()
        out, report = mapbuild.build(src, block_size=8.0)
        before, after = mf.ModFile.parse(src), mf.ModFile.parse(out)
        self.assertEqual(after.tail, before.tail)  # routes, lights
        self.assertEqual(after.get(mf.CHUNK_COLLPRISM).payload, before.get(mf.CHUNK_COLLPRISM).payload)
        joints = mf.decode_joints(after.get(mf.CHUNK_JOINT).payload)
        meshes = mf.decode_meshes(after.get(mf.CHUNK_MESH).payload)
        mats = mf.decode_materials(after.get(mf.CHUNK_MATERIAL).payload)
        attrs = mf.decode_texattrs(after.get(mf.CHUNK_TEXATTR).payload)
        texs = mf.decode_textures(after.get(mf.CHUNK_TEXTURE).payload)
        verts = mf.decode_vec3s(after.get(mf.CHUNK_VERTEX).payload)
        uvs = mf.decode_vec2s(after.get(mf.CHUNK_TEXCOORD0).payload)
        self.assertEqual(report.meshes_hidden, 1)
        self.assertNotIn((0, 0), joints[0].matpolys)  # original terrain no longer drawn
        for mat, mesh in joints[0].matpolys:
            tex = texs[attrs[mats.materials[mat].tex_attrs()[0]].texture_index]
            self.assertEqual((tex.width, tex.height), (32, 32))
            for _op, vs in mf.decode_displist(meshes[mesh].groups[0].displists[0].data, meshes[mesh].flags):
                for _m, p, _n, _c, (t,) in vs:
                    self.assertLess(p, len(verts))
                    self.assertLess(t, len(uvs))
        self.assertGreater(len(joints[0].matpolys), 3)

    def test_winding_follows_map(self):
        for ccw in (False, True):
            src = synthetic_map(ccw)
            mod = mf.ModFile.parse(src)
            meshes = mf.decode_meshes(mod.get(mf.CHUNK_MESH).payload)
            verts = mf.decode_vec3s(mod.get(mf.CHUNK_VERTEX).payload)
            self.assertEqual(mapbuild.front_is_ccw(meshes, verts, [(0, 1, 0)]), ccw)
            out, _ = mapbuild.build(src, block_size=8.0)
            mod = mf.ModFile.parse(out)
            meshes = mf.decode_meshes(mod.get(mf.CHUNK_MESH).payload)
            verts = mf.decode_vec3s(mod.get(mf.CHUNK_VERTEX).payload)
            normals = mf.decode_vec3s(mod.get(mf.CHUNK_NORMAL).payload)
            self.assertEqual(mapbuild.front_is_ccw(meshes[1:], verts, normals), ccw)


if __name__ == "__main__":
    unittest.main()
