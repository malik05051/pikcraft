"""Graft block terrain into a Pikmin map model.

The collision data, vertices, materials, lights and carrying routes all stay as they are, so
gameplay is unchanged. The joints' draw lists point at new block meshes instead of the original
terrain meshes. Every block material is a copy of one of the map's own materials with only its
texture swapped, so the game's renderer sees nothing it hasn't drawn before.
"""

from __future__ import annotations

import struct
from collections import Counter
from dataclasses import dataclass

from . import modfile as mf
from . import textures, voxel

MAX_VERTICES = 0xFFFF  # display lists index vertices with 16 bits (0..0xFFFF)

# Blend class each block texture needs, in order of preference for its template material.
BLEND = {
    "water": (mf.MATFLAG_ALPHA_BLEND, mf.MATFLAG_OPAQUE),
    "leaves": (mf.MATFLAG_ALPHA_TEST, mf.MATFLAG_OPAQUE),
}
DEFAULT_BLEND = (mf.MATFLAG_OPAQUE,)
BLEND_MASK = mf.MATFLAG_OPAQUE | mf.MATFLAG_ALPHA_TEST | mf.MATFLAG_ALPHA_BLEND


class BuildError(Exception):
    pass


@dataclass
class BuildReport:
    block_size: float
    columns: int
    quads: int
    vertices_before: int
    vertices_after: int
    templates: dict[str, int]  # block texture -> material it was copied from
    meshes_hidden: int


@dataclass
class Template:
    mesh_index: int
    material_index: int
    joint_index: int
    mesh: mf.Mesh
    group: mf.MtxGroup
    displist_flags: int
    vertex: tuple  # a vertex of the template mesh: (mtx, pos, nrm, clr, texs)


def _matpoly_owner(joints: list[mf.Joint]) -> dict[int, tuple[int, int]]:
    """mesh index -> (joint index, material index)"""
    owner = {}
    for ji, j in enumerate(joints):
        for mat, mesh in j.matpolys:
            owner[mesh] = (ji, mat)
    return owner


def _face_count(mesh: mf.Mesh) -> int:
    return sum(dl.face_count for g in mesh.groups for dl in g.displists)


def _first_vertex(mesh: mf.Mesh):
    for g in mesh.groups:
        for dl in g.displists:
            for _op, verts in mf.decode_displist(dl.data, mesh.flags):
                if verts:
                    return g, dl, verts[0]
    return None


def pick_template(meshes, joints, mats: mf.MaterialChunk, blend: int = mf.MATFLAG_OPAQUE) -> Template | None:
    """The most-drawn textured mesh whose material has one texture and the given blend class."""
    owner = _matpoly_owner(joints)
    score: Counter = Counter()
    for mi, mesh in enumerate(meshes):
        if mi not in owner or not (mesh.flags & mf.MESH_TEX0):
            continue
        mat = mats.materials[owner[mi][1]]
        if len(mat.tex_attr_offsets) != 1 or mat.flags & mf.MATFLAG_SKIP or (mat.flags & BLEND_MASK) != blend:
            continue
        score[mi] += _face_count(mesh)
    for mi, _ in score.most_common():
        first = _first_vertex(meshes[mi])
        if first:
            group, dl, vertex = first
            ji, mat = owner[mi]
            return Template(mi, mat, ji, meshes[mi], group, dl.flags, vertex)
    return None


def ground_tris(prism: mf.CollPrism) -> list[tuple]:
    """Collision triangles that can be stood on (overhangs and ceilings would become pillars)."""
    return [(t.verts[0], t.verts[1], t.verts[2], t.attribute, t.map_code) for t in prism.tris if t.plane[1] > 0.05]


def _tri_corners(prims):
    """Triangles of decoded primitives, in drawn order (strip triangles alternate winding)."""
    for op, vs in prims:
        if op == mf.GX_TRIANGLES:
            for i in range(0, len(vs) - 2, 3):
                yield vs[i], vs[i + 1], vs[i + 2]
        elif op == mf.GX_TRIANGLESTRIP:
            for i in range(len(vs) - 2):
                a, b, c = vs[i], vs[i + 1], vs[i + 2]
                yield (a, b, c) if i % 2 == 0 else (b, a, c)
        elif op == mf.GX_TRIANGLEFAN:
            for i in range(1, len(vs) - 1):
                yield vs[0], vs[i], vs[i + 1]


def front_is_ccw(meshes: list[mf.Mesh], verts, normals) -> bool:
    """Whether the map draws triangles counter-clockwise around their normals (majority vote)."""
    votes = 0
    for mesh in meshes:
        for g in mesh.groups:
            for dl in g.displists:
                for a, b, c in _tri_corners(mf.decode_displist(dl.data, mesh.flags)):
                    pa, pb, pc = verts[a[1]], verts[b[1]], verts[c[1]]
                    e1 = [pb[i] - pa[i] for i in range(3)]
                    e2 = [pc[i] - pa[i] for i in range(3)]
                    cross = (e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0])
                    n = normals[a[2]]
                    d = sum(cross[i] * n[i] for i in range(3))
                    votes += (d > 0) - (d < 0)
    return votes >= 0


def build(data: bytes, block_size: float, keep_original: bool = False) -> tuple[bytes, BuildReport]:
    mod = mf.ModFile.parse(data)

    def payload(cid):
        c = mod.get(cid)
        if c is None:
            raise BuildError(f"map has no chunk 0x{cid:x}")
        return c.payload

    verts = mf.decode_vec3s(payload(mf.CHUNK_VERTEX))
    normals = mf.decode_vec3s(payload(mf.CHUNK_NORMAL))
    colours = mf.decode_colours(mod.get(mf.CHUNK_COLOUR).payload) if mod.get(mf.CHUNK_COLOUR) else []
    texs = mf.decode_textures(payload(mf.CHUNK_TEXTURE))
    attrs = mf.decode_texattrs(payload(mf.CHUNK_TEXATTR))
    mats = mf.decode_materials(payload(mf.CHUNK_MATERIAL))
    meshes = mf.decode_meshes(payload(mf.CHUNK_MESH))
    joints = mf.decode_joints(payload(mf.CHUNK_JOINT))
    prism = mf.decode_collprism(payload(mf.CHUNK_COLLPRISM))
    ccw = front_is_ccw(meshes, verts, normals)

    # ---- terrain blocks from the collision surface
    tri_list = ground_tris(prism)
    size = block_size
    while True:
        grid = voxel.sample(verts, tri_list, size)
        quads = voxel.mesh(grid)
        corners = {c for q in quads for c in q.corners}
        if len(verts) + len(corners) <= MAX_VERTICES + 1:
            break
        size *= 1.25  # too detailed for 16-bit indices: use bigger blocks

    # ---- a template (material + mesh layout) per block texture
    names = sorted({q.texture for q in quads})
    cache: dict[int, Template | None] = {}

    def template_for(name: str) -> Template:
        for blend in BLEND.get(name, DEFAULT_BLEND):
            if blend not in cache:
                cache[blend] = pick_template(meshes, joints, mats, blend)
            if cache[blend]:
                return cache[blend]
        raise BuildError("no single-texture material to base block materials on")

    templates = {name: template_for(name) for name in names}
    slots = max(len(t.vertex[4]) for t in templates.values())
    # One UV list per texture coordinate slot the templates use (chunks 0x18..0x1f).
    uv_lists = [mf.decode_vec2s(payload(mf.CHUNK_TEXCOORD0 + n)) for n in range(slots)]

    mat_index: dict[str, int] = {}
    for name in names:
        t = templates[name]
        src = mats.materials[t.material_index]
        attr = attrs[src.tex_attrs()[0]]
        texs.append(textures.make_teximg(name))
        attrs.append(mf.TexAttr(len(texs) - 1, attr.unknown, 0, 0, attr.lod_bias))
        mats.materials.append(src.with_tex_attr(len(attrs) - 1))
        mat_index[name] = len(mats.materials) - 1

    # ---- geometry
    vert_index: dict[tuple, int] = {}
    nrm_index: dict[tuple, int] = {}
    uv_index: list[dict[tuple, int]] = [{} for _ in uv_lists]
    white = None
    if any(t.mesh.flags & mf.MESH_VTXCOLOR for t in templates.values()):
        colours.append((255, 255, 255, 255))
        white = len(colours) - 1

    def vidx(p):
        if p not in vert_index:
            vert_index[p] = len(verts)
            verts.append(p)
        return vert_index[p]

    def nidx(n):
        key = tuple(float(c) for c in n)
        if key not in nrm_index:
            nrm_index[key] = len(normals)
            normals.append(key)
        return nrm_index[key]

    def tidx(slot, uv):
        if uv not in uv_index[slot]:
            uv_index[slot][uv] = len(uv_lists[slot])
            uv_lists[slot].append(uv)
        return uv_index[slot][uv]

    by_texture: dict[str, list[tuple]] = {n: [] for n in names}
    for q in quads:
        t = templates[q.texture]
        n = nidx(q.normal)
        vs = []
        for p, uv in zip(q.corners, q.uvs):
            vs.append((t.vertex[0], vidx(p), n, white, tuple(tidx(s, uv) for s in range(len(t.vertex[4])))))
        # Quads are counter-clockwise seen from outside; match the map's own front-face winding.
        if ccw:
            by_texture[q.texture] += [vs[0], vs[1], vs[2], vs[0], vs[2], vs[3]]
        else:
            by_texture[q.texture] += [vs[0], vs[2], vs[1], vs[0], vs[3], vs[2]]
    if any(len(u) > MAX_VERTICES + 1 for u in uv_lists) or len(normals) > MAX_VERTICES + 1:
        raise BuildError("map uses too many texture coordinates or normals for 16-bit indices")

    # ---- new meshes, drawn from each template's joint
    hidden = 0
    if not keep_original:
        for j in joints:
            hidden += len(j.matpolys)
            j.matpolys = []
    for name in names:
        t = templates[name]
        dl = mf.encode_triangles(by_texture[name], t.mesh.flags)
        group = mf.MtxGroup(list(t.group.deps), [mf.DispList(t.displist_flags, len(by_texture[name]) // 3, dl)])
        meshes.append(mf.Mesh(t.mesh.parent_joint, t.mesh.flags, [group]))
        joints[t.joint_index].matpolys.append((mat_index[name], len(meshes) - 1))

    lo = tuple(min(p[i] for p in vert_index) for i in range(3))
    hi = tuple(max(p[i] for p in vert_index) for i in range(3))
    for ji in {t.joint_index for t in templates.values()}:
        j = joints[ji]
        j.bounds_min = tuple(min(j.bounds_min[i], lo[i]) for i in range(3))
        j.bounds_max = tuple(max(j.bounds_max[i], hi[i]) for i in range(3))

    mod.set(mf.CHUNK_VERTEX, mf.encode_vec3s(verts))
    mod.set(mf.CHUNK_NORMAL, mf.encode_vec3s(normals))
    for slot, uvs in enumerate(uv_lists):
        mod.set(mf.CHUNK_TEXCOORD0 + slot, mf.encode_vec2s(uvs))
    if white is not None:
        mod.set(mf.CHUNK_COLOUR, mf.encode_colours(colours))
    mod.set(mf.CHUNK_TEXTURE, mf.encode_textures(texs))
    mod.set(mf.CHUNK_TEXATTR, mf.encode_texattrs(attrs))
    mod.set(mf.CHUNK_MATERIAL, mf.encode_materials(mats))
    mod.set(mf.CHUNK_MESH, mf.encode_meshes(meshes))
    mod.set(mf.CHUNK_JOINT, mf.encode_joints(joints))

    report = BuildReport(
        size,
        sum(1 for c in grid.columns if c.ground is not None),
        len(quads),
        len(verts) - len(vert_index),
        len(verts),
        {name: t.material_index for name, t in templates.items()},
        hidden,
    )
    return mod.to_bytes(), report


# ----------------------------------------------------------------------------- block chunk

CHUNK_BLOCKS = 0x5043  # read by the PikCraft game code (include/PikCraft.h)
CHUNK_MAGIC = 0x50434231  # 'PCB1'
NO_GROUND = -32768


def _align(buf: bytearray, n: int = 32) -> int:
    buf += b"\0" * (-len(buf) % n)
    return len(buf)


def block_chunk(grid: voxel.Grid) -> bytes:
    """The block heightmap the game code draws, collides with and edits."""
    n = grid.nx * grid.nz
    ground = [c.ground if c.ground is not None else NO_GROUND for c in grid.columns]
    water = [c.water if c.water is not None and c.ground is not None else NO_GROUND for c in grid.columns]
    surface = [c.surface if c.ground is not None else voxel.AIR for c in grid.columns]
    codes = [((c.code >> 29) & 7) | (((c.code >> 27) & 3) << 3) | (((c.code >> 25) & 1) << 5) for c in grid.columns]
    top = max(max(g for g in ground if g != NO_GROUND), max((w for w in water if w != NO_GROUND), default=-32768))

    body = bytearray(64)
    offsets = []
    for fmt, values in ((">%dh" % n, ground), (">%dh" % n, water), ("%dB" % n, surface), ("%dB" % n, codes)):
        offsets.append(_align(body))
        body += struct.pack(fmt, *values)
    offsets.append(_align(body))
    atlas = textures.atlas()
    body += atlas
    body += b"\0" * (-(len(body) + 8) % 32)  # the next chunk (after this one's 8-byte header) starts aligned
    struct.pack_into(
        ">II4f4i6I",
        body,
        0,
        CHUNK_MAGIC,
        1,
        grid.origin_x,
        grid.origin_y,
        grid.origin_z,
        grid.size,
        grid.nx,
        grid.nz,
        grid.floor,
        top,
        *offsets,
        len(atlas),
    )
    return bytes(body)


def build_blocks(data: bytes, block_size: float) -> tuple[bytes, BuildReport]:
    """Map model for the PikCraft game code: the original terrain is hidden and the block heightmap is
    added; the game draws it and answers collision from it."""
    mod = mf.ModFile.parse(data)
    prism = mod.get(mf.CHUNK_COLLPRISM)
    joints_chunk = mod.get(mf.CHUNK_JOINT)
    if prism is None or joints_chunk is None:
        raise BuildError("map has no collision or joints")
    verts = mf.decode_vec3s(mod.get(mf.CHUNK_VERTEX).payload)
    tris = ground_tris(mf.decode_collprism(prism.payload))
    grid = voxel.sample(verts, tris, block_size)

    joints = mf.decode_joints(joints_chunk.payload)
    hidden = 0
    for j in joints:
        hidden += len(j.matpolys)
        j.matpolys = []
    mod.set(mf.CHUNK_JOINT, mf.encode_joints(joints))
    # The terrain textures are never drawn now: keep tiny stand-ins (materials still refer to them)
    # so the blocks have plenty of memory.
    texs = mf.decode_textures(mod.get(mf.CHUNK_TEXTURE).payload)
    stand_in = mf.TexImg(8, 8, textures.FMT_RGB565, 1, (0, 0, 0, 0), b"\x84\x10" * 64)
    mod.set(mf.CHUNK_TEXTURE, mf.encode_textures([stand_in for _ in texs]))
    mod.set(CHUNK_BLOCKS, block_chunk(grid))
    report = BuildReport(
        block_size, sum(1 for c in grid.columns if c.ground is not None), 0, len(verts), len(verts), {}, hidden
    )
    return mod.to_bytes(), report
