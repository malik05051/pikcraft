"""Voxelizer tests on synthetic terrain."""

import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from pikcraft_patcher import textures, voxel  # noqa: E402


def terrain(n=20, step=10.0, pond=True):
    """A gentle slope (height = x/4) as a triangle grid, plus an optional pond surface."""
    verts, tris = [], []

    def h(x, z):
        base = x * 0.25
        if pond and 60 <= x <= 120 and 60 <= z <= 120:
            base -= 30  # pond bed
        return base

    for k in range(n + 1):
        for i in range(n + 1):
            verts.append((i * step, h(i * step, k * step), k * step))
    for k in range(n):
        for i in range(n):
            a = k * (n + 1) + i
            b, c, d = a + 1, a + n + 1, a + n + 2
            attr = voxel.ATTR_GRASS
            tris.append((a, c, b, attr))
            tris.append((b, c, d, attr))
    if pond:
        base = len(verts)
        verts += [(60, 10, 60), (120, 10, 60), (60, 10, 120), (120, 10, 120)]
        tris.append((base, base + 2, base + 1, voxel.ATTR_WATER))
        tris.append((base + 1, base + 2, base + 3, voxel.ATTR_WATER))
    return verts, tris


class VoxelTests(unittest.TestCase):
    def test_heights_follow_surface(self):
        verts, tris = terrain(pond=False)
        grid = voxel.sample(verts, tris, block=8.0)
        index = voxel.TriIndex(verts, tris, 32.0)
        for k in range(grid.nz):
            for i in range(grid.nx):
                col = grid.col(i, k)
                if col.ground is None:
                    continue
                x = grid.origin_x + (i + 0.5) * grid.size
                z = grid.origin_z + (k + 0.5) * grid.size
                true = index.hits(x, z)[0][0]
                top = grid.origin_y + (col.ground + 1) * grid.size
                self.assertLessEqual(abs(top - true), grid.size / 2 + 1e-6)

    def test_pond_fills_with_water_over_sand(self):
        verts, tris = terrain()
        grid = voxel.sample(verts, tris, block=8.0)
        i = int((90 - grid.origin_x) / grid.size)
        k = int((90 - grid.origin_z) / grid.size)
        col = grid.col(i, k)
        self.assertIsNotNone(col.water)
        self.assertGreater(col.water, col.ground)
        self.assertEqual(col.surface, voxel.SAND)
        self.assertEqual(voxel.block_at(grid, i, col.water, k), voxel.WATER)

    def test_water_marked_pond_bed_fills_to_shore(self):
        # As in Pikmin's maps: the pond bed is collision marked "water" and there is no surface.
        verts, tris = terrain(pond=False)
        for x in range(len(verts)):
            vx, vy, vz = verts[x]
            if 60 <= vx <= 120 and 60 <= vz <= 120:
                verts[x] = (vx, vy - 40, vz)
        tris = [
            (a, b, c, voxel.ATTR_WATER if all(60 <= verts[v][0] <= 120 and 60 <= verts[v][2] <= 120 for v in (a, b, c)) else t)
            for a, b, c, t in tris
        ]
        grid = voxel.sample(verts, tris, block=8.0)
        i = int((90 - grid.origin_x) / grid.size)
        k = int((90 - grid.origin_z) / grid.size)
        col = grid.col(i, k)
        self.assertIsNotNone(col.ground)
        self.assertEqual(col.surface, voxel.SAND)
        self.assertIsNotNone(col.water)
        self.assertGreater(col.water, col.ground)
        # No holes where the bed is marked water.
        for kk in range(1, grid.nz - 1):
            for ii in range(1, grid.nx - 1):
                self.assertIsNotNone(grid.col(ii, kk).ground)

    def test_greedy_mesh_merges_faces(self):
        verts, tris = terrain(pond=False)
        grid = voxel.sample(verts, tris, block=8.0)
        quads = voxel.mesh(grid)
        filled = sum(1 for c in grid.columns if c.ground is not None)
        tops = [q for q in quads if q.normal == (0, 1, 0)]
        # The slope makes long terraces, so merged top quads are far fewer than columns.
        self.assertLess(len(tops), filled / 3)
        # Every quad's corners are on the block lattice.
        for q in quads:
            for cx, cy, cz in q.corners:
                for v, o in ((cx, grid.origin_x), (cy, grid.origin_y), (cz, grid.origin_z)):
                    self.assertAlmostEqual(((v - o) / grid.size) % 1.0, 0.0, places=5)

    def test_quads_wind_outwards(self):
        verts, tris = terrain(pond=False, n=6)
        grid = voxel.sample(verts, tris, block=8.0)
        for q in voxel.mesh(grid):
            a, b, c = q.corners[0], q.corners[1], q.corners[2]
            e1 = [b[i] - a[i] for i in range(3)]
            e2 = [c[i] - a[i] for i in range(3)]
            cross = (e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0])
            self.assertGreater(sum(cross[i] * q.normal[i] for i in range(3)), 0)

    def test_textures_round_trip(self):
        for name in textures.BLOCK_TEXTURES:
            img = textures.make_teximg(name, 16)
            px = textures.decode(img.data, 16, 16, img.fmt)
            self.assertEqual(len(img.data), 16 * 16 * 2)
            src = textures.pixels(name, 16)
            for a, b in zip(px, src):
                for ca, cb in zip(a[:3], b[:3]):
                    self.assertLessEqual(abs(ca - cb), 17)


if __name__ == "__main__":
    unittest.main()
