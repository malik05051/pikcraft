"""Graft block terrain into a Pikmin map model.

The original collision triangles, vertices, materials, lights and carrying routes stay as they are
(gameplay is unchanged); the joints' draw lists are pointed at new block meshes instead of the
original terrain meshes.
"""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass

from . import modfile as mf
from . import textures, voxel

MAX_VERTICES = 0xFFFF  # display lists index vertices with 16 bits (0..0xFFFF)


class BuildError(Exception):
    pass


@dataclass
class BuildReport:
    block_size: float
    columns: int
    quads: int
    vertices_before: int
    vertices_after: int
    template_material: int
    meshes_hidden: int


def _matpoly_owner(joints: list[mf.Joint]) -> dict[int, tuple[int, int]]:
    """mesh index -> (joint index, material index)"""
    owner = {}
    for ji, j in enumerate(joints):
        for mat, mesh in j.matpolys:
            owner[mesh] = (ji, mat)
    return owner


def _face_count(mesh: mf.Mesh) -> int:
    return sum(dl.face_count for g in mesh.groups for dl in g.displists)


def pick_template(meshes: list[mf.Mesh], joints: list[mf.Joint], mats: mf.MaterialChunk) -> tuple[int, int]:
    """(mesh index, material index) of the most-drawn single-texture terrain material."""
    owner = _matpoly_owner(joints)
    score: Counter = Counter()
    for mi, mesh in enumerate(meshes):
        if mi not in owner or not (mesh.flags & mf.MESH_TEX0):
            continue
        mat = owner[mi][1]
        if len(mats.materials[mat].tex_attr_offsets) != 1:
            continue
        score[(mi, mat)] += _face_count(mesh)
    if not score:
        raise BuildError("no single-texture material to base block materials on")
    return score.most_common(1)[0][0]


def _first_vertex(mesh: mf.Mesh):
    for g in mesh.groups:
        for dl in g.displists:
            for _op, verts in mf.decode_displist(dl.data, mesh.flags):
                if verts:
                    return g, dl, verts[0]
    raise BuildError("template mesh has no vertices")


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

    # ---- terrain blocks from the collision surface
    tri_list = [(t.verts[0], t.verts[1], t.verts[2], t.attribute) for t in prism.tris]
    size = block_size
    while True:
        grid = voxel.sample(verts, tri_list, size)
        quads = voxel.mesh(grid)
        corners = {c for q in quads for c in q.corners}
        if len(verts) + len(corners) <= MAX_VERTICES + 1:
            break
        size *= 1.25  # too detailed for 16-bit indices: use bigger blocks

    template_mesh_idx, template_mat_idx = pick_template(meshes, joints, mats)
    template_mesh = meshes[template_mesh_idx]
    owner_joint = _matpoly_owner(joints)[template_mesh_idx][0]
    t_group, _t_dl, t_vertex = _first_vertex(template_mesh)
    t_mtx, _pos, _nrm, _clr, t_tex = t_vertex
    ccw = front_is_ccw(meshes, verts, normals)
    # One UV list per texture coordinate slot the template mesh uses (chunks 0x18..0x1f).
    uv_lists = [mf.decode_vec2s(payload(mf.CHUNK_TEXCOORD0 + n)) for n in range(len(t_tex))]

    # ---- textures, attributes and cloned materials per block texture
    names = sorted({q.texture for q in quads})
    tex_attr_template = attrs[mats.materials[template_mat_idx].tex_attrs()[0]]
    mat_index: dict[str, int] = {}
    for name in names:
        texs.append(textures.make_teximg(name))
        attrs.append(mf.TexAttr(len(texs) - 1, tex_attr_template.unknown, 0, 0, tex_attr_template.lod_bias))
        mats.materials.append(mats.materials[template_mat_idx].with_tex_attr(len(attrs) - 1))
        mat_index[name] = len(mats.materials) - 1

    # ---- geometry
    vert_index: dict[tuple, int] = {}
    nrm_index: dict[tuple, int] = {}
    uv_index: list[dict[tuple, int]] = [{} for _ in uv_lists]
    white = None
    if template_mesh.flags & mf.MESH_VTXCOLOR:
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
        n = nidx(q.normal)
        vs = []
        for p, uv in zip(q.corners, q.uvs):
            vs.append((t_mtx, vidx(p), n, white, tuple(tidx(slot, uv) for slot in range(len(uv_lists)))))
        # Quads are counter-clockwise seen from outside; match the map's own front-face winding.
        if ccw:
            by_texture[q.texture] += [vs[0], vs[1], vs[2], vs[0], vs[2], vs[3]]
        else:
            by_texture[q.texture] += [vs[0], vs[2], vs[1], vs[0], vs[3], vs[2]]
    if any(len(u) > MAX_VERTICES + 1 for u in uv_lists) or len(normals) > MAX_VERTICES + 1:
        raise BuildError("map uses too many texture coordinates or normals for 16-bit indices")

    new_meshes = []
    for name in names:
        dl = mf.encode_triangles(by_texture[name], template_mesh.flags)
        faces = len(by_texture[name]) // 3
        new_meshes.append(
            mf.Mesh(
                template_mesh.parent_joint,
                template_mesh.flags,
                [mf.MtxGroup(list(t_group.deps), [mf.DispList(template_mesh.groups[0].displists[0].flags, faces, dl)])],
            )
        )

    # ---- point the draw lists at the blocks
    hidden = 0
    first_new_mesh = len(meshes)
    meshes += new_meshes
    if not keep_original:
        for j in joints:
            hidden += len(j.matpolys)
            j.matpolys = []
    joint = joints[owner_joint]
    joint.matpolys += [(mat_index[name], first_new_mesh + i) for i, name in enumerate(names)]
    xs = [p[0] for p in vert_index]
    ys = [p[1] for p in vert_index]
    zs = [p[2] for p in vert_index]
    joint.bounds_min = (min(joint.bounds_min[0], *xs), min(joint.bounds_min[1], *ys), min(joint.bounds_min[2], *zs))
    joint.bounds_max = (max(joint.bounds_max[0], *xs), max(joint.bounds_max[1], *ys), max(joint.bounds_max[2], *zs))

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
        template_mat_idx,
        hidden,
    )
    return mod.to_bytes(), report
