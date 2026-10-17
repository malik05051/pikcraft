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
    wet: set[tuple[int, int]] = set()  # pond-bed columns whose water level comes from the shore
    lowest = None
    for k in range(nz):
        for i in range(nx):
            x = ox + (i + 0.5) * block
            z = oz + (k + 0.5) * block
            hits = index.hits(x, z)
            if not hits:
                columns.append(Column(None, None, AIR))
                continue
            ground = next(((h, a) for h, a in hits if a != ATTR_WATER), None)
            water = next((h for h, a in hits if a == ATTR_WATER), None)
            w = None
            if ground is None or (water is not None and abs(water - ground[0]) <= block * 0.25):
                # Pikmin marks the pond bed itself as water; the visible surface is not collision.
                ground = (water, ATTR_WATER)
                wet.add((i, k))
            elif water is not None and water > ground[0] + block * 0.25:
                w = int(round((water - oy) / block)) - 1
            g = int(round((ground[0] - oy) / block)) - 1  # top layer whose top face is at the surface
            if w is not None and w <= g:
                w = None
            surface = SAND if w is not None or (i, k) in wet else surface_block(ground[1])
            columns.append(Column(g, w, surface))
            lowest = g if lowest is None else min(lowest, g)
    grid = Grid(ox, oz, oy, block, nx, nz, columns, (lowest or 0) - 2)
    _fill_ponds(grid, wet)
    return grid


def _fill_ponds(grid: Grid, wet: set[tuple[int, int]]) -> None:
    """Fill each connected pond bed with water up to (nearly) the lowest dry ground around it."""
    seen: set[tuple[int, int]] = set()
    for start in wet:
        if start in seen:
            continue
        pond, rim, todo = [], [], [start]
        seen.add(start)
        while todo:
            i, k = todo.pop()
            pond.append((i, k))
            for n in ((i + 1, k), (i - 1, k), (i, k + 1), (i, k - 1)):
                c = grid.col(*n)
                if c is None or c.ground is None:
                    continue
                if n in wet:
                    if n not in seen:
                        seen.add(n)
                        todo.append(n)
                else:
                    rim.append(c.ground)
        if not rim:
            continue
        rim.sort()
        level = rim[len(rim) // 4]  # a low shore point, robust to the odd stray column
        for i, k in pond:
            c = grid.col(i, k)
            if level > c.ground:
                c.water = level


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


def _faces(grid: Grid, include_bottom: bool):
    """Visible block faces as {direction: {slice: {(a, b): texture}}}, visiting only exposed heights.

    Coordinates are (x, y - floor, z) projected onto each direction's (axis, u, v) axes.
    """
    ylo = grid.floor
    out: dict[str, dict[int, dict[tuple[int, int], str]]] = {name: {} for name in DIRS}
    axes = {}
    for name, (n, u, v) in DIRS.items():
        axes[name] = ([i for i in range(3) if n[i] != 0][0], [i for i in range(3) if u[i] != 0][0], [i for i in range(3) if v[i] != 0][0])

    def add(name, x, y, z, blk):
        kind = "top" if name == "+y" else "bottom" if name == "-y" else "side"
        axis, ua, va = axes[name]
        p = (x, y - ylo, z)
        out[name].setdefault(p[axis], {})[(p[ua], p[va])] = face_texture(blk, kind)

    def solid_top(c):  # highest solid layer of a column (or below the floor)
        return c.ground if c is not None and c.ground is not None else ylo - 1

    def wet_top(c):  # highest non-air layer of a column
        if c is None or c.ground is None:
            return ylo - 1
        return max(c.ground, c.water if c.water is not None else c.ground)

    for k in range(grid.nz):
        for i in range(grid.nx):
            c = grid.col(i, k)
            if c is None or c.ground is None:
                continue
            top = wet_top(c)
            # Up: the ground's top (under air or water) and the water surface.
            add("+y", i, c.ground, k, block_at(grid, i, c.ground, k))
            if c.water is not None:
                add("+y", i, c.water, k, WATER)
            if include_bottom:
                add("-y", i, ylo, k, block_at(grid, i, ylo, k))
            # Sides: solid blocks show above the neighbour's ground; water only above its water.
            for name, di, dk in (("+x", 1, 0), ("-x", -1, 0), ("+z", 0, 1), ("-z", 0, -1)):
                nb = grid.col(i + di, k + dk)
                for y in range(max(solid_top(nb), ylo - 1) + 1, top + 1):
                    blk = block_at(grid, i, y, k)
                    if blk == WATER and y <= wet_top(nb):
                        continue
                    add(name, i, y, k, blk)
    return out


def mesh(grid: Grid, include_bottom: bool = False) -> list[Quad]:
    """Greedy-meshed visible faces of the block grid."""
    quads: list[Quad] = []
    for name, slices in _faces(grid, include_bottom).items():
        n = DIRS[name][0]
        axis = [i for i in range(3) if n[i] != 0][0]
        ua = [i for i in range(3) if DIRS[name][1][i] != 0][0]
        va = [i for i in range(3) if DIRS[name][2][i] != 0][0]
        sgn = n[axis]
        for d, mask in sorted(slices.items()):
            # Greedy merge rectangles of equal texture.
            done = set()
            for a, b in sorted(mask):
                if (a, b) in done:
                    continue
                tex = mask[(a, b)]
                w = 1
                while mask.get((a, b + w)) == tex and (a, b + w) not in done:
                    w += 1
                h = 1
                while all(mask.get((a + h, b + j)) == tex and (a + h, b + j) not in done for j in range(w)):
                    h += 1
                for da in range(h):
                    for db in range(w):
                        done.add((a + da, b + db))
                quads.append(_make_quad(grid, grid.floor, axis, ua, va, sgn, d, a, b, h, w, tex, n))
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
