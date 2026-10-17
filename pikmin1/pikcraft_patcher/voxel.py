"""Turn a Pikmin map's collision surface into Minecraft-style block terrain.

Sampling: every column of the block grid is probed with a vertical ray against the map's collision
triangles. The highest non-water hit is the ground; a water hit above it is a pond surface. Ground
heights are rounded to whole blocks, so the visible terrain stays within half a block of the
surface Pikmin actually walk on (the original collision mesh is kept for gameplay).

Meshing: visible faces are merged greedily per block type and direction, so large flat areas cost
four vertices instead of thousands.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

# Map surface attributes (include/MapCode.h)
ATTR_SOLID, ATTR_ROCK, ATTR_GRASS, ATTR_WOOD, ATTR_MUD, ATTR_WATER, ATTR_HOLE = range(7)

AIR, GRASS, DIRT, STONE, SAND, LOG, PLANKS, LEAVES, WATER, MUD = range(10)
BLOCK_NAMES = ["air", "grass", "dirt", "stone", "sand", "log", "planks", "leaves", "water", "mud"]

# Texture used for each (block, face kind) where kind is "top", "bottom" or "side".
FACE_TEXTURE = {
    GRASS: {"top": "grass_top", "bottom": "dirt", "side": "grass_side"},
    DIRT: "dirt",
    STONE: "stone",
    SAND: "sand",
    LOG: "log",
    PLANKS: "planks",
    LEAVES: "leaves",
    WATER: "water",
    MUD: "mud",
}


def face_texture(block: int, kind: str) -> str:
    t = FACE_TEXTURE[block]
    return t[kind] if isinstance(t, dict) else t


@dataclass
class Column:
    ground: int | None  # top solid block layer (inclusive), None = no ground here
    water: int | None  # top water layer (inclusive) above the ground, if any
    surface: int  # block type of the top ground layer


@dataclass
class Grid:
    origin_x: float
    origin_z: float
    origin_y: float
    size: float
    nx: int
    nz: int
    columns: list[Column]
    floor: int  # lowest layer drawn (sides stop here)

    def col(self, i: int, k: int) -> Column | None:
        if 0 <= i < self.nx and 0 <= k < self.nz:
            return self.columns[k * self.nx + i]
        return None


# ----------------------------------------------------------------------------- sampling


class TriIndex:
    """Uniform XZ bucket grid over triangles for fast vertical ray casts."""

    def __init__(self, verts, tris, cell: float):
        self.verts = verts
        self.tris = tris
        self.cell = cell
        self.buckets: dict[tuple[int, int], list[int]] = {}
        for ti, (a, b, c, _attr) in enumerate(tris):
            xs = (verts[a][0], verts[b][0], verts[c][0])
            zs = (verts[a][2], verts[b][2], verts[c][2])
            for bx in range(int(math.floor(min(xs) / cell)), int(math.floor(max(xs) / cell)) + 1):
                for bz in range(int(math.floor(min(zs) / cell)), int(math.floor(max(zs) / cell)) + 1):
                    self.buckets.setdefault((bx, bz), []).append(ti)

    def hits(self, x: float, z: float) -> list[tuple[float, int]]:
        """(height, attribute) of every triangle above or below (x, z), highest first."""
        out = []
        verts = self.verts
        for ti in self.buckets.get((int(math.floor(x / self.cell)), int(math.floor(z / self.cell))), ()):
            a, b, c, attr = self.tris[ti]
            (x1, y1, z1), (x2, y2, z2), (x3, y3, z3) = verts[a], verts[b], verts[c]
            det = (z2 - z3) * (x1 - x3) + (x3 - x2) * (z1 - z3)
            if abs(det) < 1e-9:
                continue  # vertical wall
            l1 = ((z2 - z3) * (x - x3) + (x3 - x2) * (z - z3)) / det
            l2 = ((z3 - z1) * (x - x3) + (x1 - x3) * (z - z3)) / det
            l3 = 1.0 - l1 - l2
            if l1 < -1e-6 or l2 < -1e-6 or l3 < -1e-6:
                continue
            out.append((l1 * y1 + l2 * y2 + l3 * y3, attr))
        out.sort(reverse=True)
        return out


def surface_block(attr: int) -> int:
    return {
        ATTR_ROCK: STONE,
        ATTR_GRASS: GRASS,
        ATTR_WOOD: PLANKS,
        ATTR_MUD: MUD,
    }.get(attr, GRASS)


def sample(verts, tris, block: float, margin: int = 1) -> Grid:
    """Build the block grid. `tris` is a list of (v0, v1, v2, attribute)."""
    used = {i for t in tris for i in t[:3]}
    xs = [verts[i][0] for i in used]
    ys = [verts[i][1] for i in used]
    zs = [verts[i][2] for i in used]
    ox = math.floor(min(xs) / block) * block - margin * block
    oz = math.floor(min(zs) / block) * block - margin * block
    oy = math.floor(min(ys) / block) * block
    nx = int(math.ceil((max(xs) - ox) / block)) + margin
    nz = int(math.ceil((max(zs) - oz) / block)) + margin
    index = TriIndex(verts, tris, block * 4)
    columns = []
    lowest = None
    for k in range(nz):
        for i in range(nx):
            x = ox + (i + 0.5) * block
            z = oz + (k + 0.5) * block
            hits = index.hits(x, z)
            ground = next(((h, a) for h, a in hits if a != ATTR_WATER), None)
            water = next((h for h, a in hits if a == ATTR_WATER), None)
            if ground is None:
                columns.append(Column(None, None, AIR))
                continue
            g = int(round((ground[0] - oy) / block)) - 1  # top layer whose top face is at the surface
            w = None
            if water is not None and water > ground[0] + block * 0.25:
                w = int(round((water - oy) / block)) - 1
                if w <= g:
                    w = None
            surface = SAND if w is not None else surface_block(ground[1])
            columns.append(Column(g, w, surface))
            lowest = g if lowest is None else min(lowest, g)
    return Grid(ox, oz, oy, block, nx, nz, columns, (lowest or 0) - 2)


def block_at(grid: Grid, i: int, y: int, k: int) -> int:
    c = grid.col(i, k)
    if c is None or c.ground is None or y < grid.floor:
        return AIR
    if y <= c.ground:
        if y == c.ground:
            return c.surface
        depth = c.ground - y
        if c.surface in (STONE, PLANKS):
            return c.surface
        return DIRT if depth < 3 else STONE
    if c.water is not None and y <= c.water:
        return WATER
    return AIR


# ----------------------------------------------------------------------------- meshing

# Face directions: normal, and the two in-plane axes (u, v) chosen so quads wind counter-clockwise
# when seen from outside.
DIRS = {
    "+y": ((0, 1, 0), (0, 0, 1), (1, 0, 0)),
    "-y": ((0, -1, 0), (1, 0, 0), (0, 0, 1)),
    "+x": ((1, 0, 0), (0, 1, 0), (0, 0, 1)),
    "-x": ((-1, 0, 0), (0, 0, 1), (0, 1, 0)),
    "+z": ((0, 0, 1), (1, 0, 0), (0, 1, 0)),
    "-z": ((0, 0, -1), (0, 1, 0), (1, 0, 0)),
}


@dataclass
class Quad:
    texture: str
    normal: tuple[int, int, int]
    corners: list[tuple[float, float, float]]  # 4 corners, CCW from outside
    uvs: list[tuple[float, float]]  # texture repeats once per block


def _visible(block: int, neighbour: int) -> bool:
    if block == AIR:
        return False
    if block == WATER:
        return neighbour == AIR
    return neighbour in (AIR, WATER, LEAVES)


def mesh(grid: Grid, include_bottom: bool = False) -> list[Quad]:
    """Greedy-meshed visible faces of the block grid."""
    top = max((max(c.ground, c.water or c.ground) for c in grid.columns if c.ground is not None), default=0) + 1
    ylo, yhi = grid.floor, top
    quads: list[Quad] = []
    size = (grid.nx, yhi - ylo + 1, grid.nz)

    def get(x, y, z):
        return block_at(grid, x, y + ylo, z)

    for name, (n, u, v) in DIRS.items():
        if name == "-y" and not include_bottom:
            continue
        axis = [i for i in range(3) if n[i] != 0][0]
        ua = [i for i in range(3) if u[i] != 0][0]
        va = [i for i in range(3) if v[i] != 0][0]
        sgn = n[axis]
        kind = "top" if name == "+y" else "bottom" if name == "-y" else "side"
        for d in range(size[axis]):
            # Mask of face textures on this slice.
            mask: dict[tuple[int, int], str] = {}
            for a in range(size[ua]):
                for b in range(size[va]):
                    p = [0, 0, 0]
                    p[axis], p[ua], p[va] = d, a, b
                    q = list(p)
                    q[axis] += sgn
                    blk = get(*p)
                    if blk != AIR and _visible(blk, get(*q)):
                        mask[(a, b)] = face_texture(blk, kind)
            # Greedy merge rectangles of equal texture.
            done = set()
            for a in range(size[ua]):
                for b in range(size[va]):
                    if (a, b) in done or (a, b) not in mask:
                        continue
                    tex = mask[(a, b)]
                    w = 1
                    while (a, b + w) in mask and mask[(a, b + w)] == tex and (a, b + w) not in done:
                        w += 1
                    h = 1
                    while all(
                        (a + h, b + j) in mask and mask[(a + h, b + j)] == tex and (a + h, b + j) not in done
                        for j in range(w)
                    ):
                        h += 1
                    for da in range(h):
                        for db in range(w):
                            done.add((a + da, b + db))
                    quads.append(_make_quad(grid, ylo, axis, ua, va, sgn, d, a, b, h, w, tex, n))
    return quads


def _make_quad(grid, ylo, axis, ua, va, sgn, d, a, b, h, w, tex, n) -> Quad:
    s = grid.size
    plane = d + (1 if sgn > 0 else 0)
    corners = []
    uvs = []
    for da, db in ((0, 0), (h, 0), (h, w), (0, w)):
        p = [0.0, 0.0, 0.0]
        p[axis] = plane
        p[ua] = a + da
        p[va] = b + db
        corners.append((grid.origin_x + p[0] * s, grid.origin_y + (p[1] + ylo) * s, grid.origin_z + p[2] * s))
        # Side textures keep "up" pointing up so grass edges sit on top.
        if axis == 1:
            uvs.append((float(db if ua == 2 else da), float(da if ua == 2 else db)))
        else:
            up = da if ua == 1 else db
            along = db if ua == 1 else da
            uvs.append((float(along), float(-up)))
    return Quad(tex, n, corners, uvs)
