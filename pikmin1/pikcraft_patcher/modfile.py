"""Pikmin 1 `.mod` model files: lossless chunk container plus typed codecs for the chunks we rebuild.

Layout follows `BaseShape::read` in the projectPiki/pikmin decompilation (src/sysCommon/shapeBase.cpp):
every chunk is `u32 id, u32 length, payload`, starts on a 32-byte boundary, and fields inside are
big-endian with 32-byte alignment after each count and at the end of the chunk. Anything after the
end-of-file chunk is an ini text block (light groups, collision infos and the carrying routes).
"""

from __future__ import annotations

import struct
from dataclasses import dataclass, field

CHUNK_HEADER = 0x00
CHUNK_VERTEX = 0x10
CHUNK_NORMAL = 0x11
CHUNK_NBT = 0x12
CHUNK_COLOUR = 0x13
CHUNK_TEXCOORD0 = 0x18
CHUNK_TEXTURE = 0x20
CHUNK_TEXATTR = 0x22
CHUNK_MATERIAL = 0x30
CHUNK_VTXMATRIX = 0x40
CHUNK_ENVELOPE = 0x41
CHUNK_MESH = 0x50
CHUNK_JOINT = 0x60
CHUNK_JOINTNAME = 0x61
CHUNK_COLLPRISM = 0x100
CHUNK_COLLGRID = 0x110
CHUNK_EOF = 0xFFFF

MATFLAG_PVW = 0x1  # material carries the full PVW (TEV/colour/texture) description

# Mesh feature flags (include/Mesh.h)
MESH_POS_NRM = 1 << 0  # per-vertex matrix index byte is present
MESH_TEX1MTX = 1 << 1
MESH_VTXCOLOR = 1 << 2
MESH_TEX0 = 1 << 3

GX_TRIANGLES = 0x90
GX_TRIANGLESTRIP = 0x98


class ModError(Exception):
    pass


class Reader:
    """Big-endian reader that tracks the absolute file position for 32-byte padding."""

    def __init__(self, data: bytes, base: int = 0):
        self.data = data
        self.pos = 0
        self.base = base

    def u8(self) -> int:
        v = self.data[self.pos]
        self.pos += 1
        return v

    def s16(self) -> int:
        v = struct.unpack_from(">h", self.data, self.pos)[0]
        self.pos += 2
        return v

    def u16(self) -> int:
        v = struct.unpack_from(">H", self.data, self.pos)[0]
        self.pos += 2
        return v

    def s32(self) -> int:
        v = struct.unpack_from(">i", self.data, self.pos)[0]
        self.pos += 4
        return v

    def u32(self) -> int:
        v = struct.unpack_from(">I", self.data, self.pos)[0]
        self.pos += 4
        return v

    def f32(self) -> float:
        v = struct.unpack_from(">f", self.data, self.pos)[0]
        self.pos += 4
        return v

    def raw(self, n: int) -> bytes:
        v = self.data[self.pos : self.pos + n]
        if len(v) != n:
            raise ModError("unexpected end of chunk")
        self.pos += n
        return v

    def pad(self, align: int = 0x20) -> None:
        absolute = self.base + self.pos
        self.pos += (align - absolute % align) % align


class Writer:
    def __init__(self, base: int = 8):
        self.buf = bytearray()
        self.base = base

    def u8(self, v: int) -> None:
        self.buf += struct.pack(">B", v & 0xFF)

    def s16(self, v: int) -> None:
        self.buf += struct.pack(">h", v)

    def u16(self, v: int) -> None:
        self.buf += struct.pack(">H", v & 0xFFFF)

    def s32(self, v: int) -> None:
        self.buf += struct.pack(">i", v)

    def u32(self, v: int) -> None:
        self.buf += struct.pack(">I", v & 0xFFFFFFFF)

    def f32(self, v: float) -> None:
        self.buf += struct.pack(">f", v)

    def raw(self, b: bytes) -> None:
        self.buf += b

    def pad(self, align: int = 0x20) -> None:
        absolute = self.base + len(self.buf)
        self.buf += b"\0" * ((align - absolute % align) % align)


# ----------------------------------------------------------------------------- container


@dataclass
class Chunk:
    id: int
    payload: bytes


@dataclass
class ModFile:
    chunks: list[Chunk]
    tail: bytes = b""

    @classmethod
    def parse(cls, data: bytes) -> "ModFile":
        pos = 0
        chunks: list[Chunk] = []
        while True:
            if pos + 8 > len(data):
                raise ModError("missing end-of-file chunk")
            cid, length = struct.unpack_from(">II", data, pos)
            payload = data[pos + 8 : pos + 8 + length]
            if len(payload) != length:
                raise ModError(f"chunk 0x{cid:x} runs past the end of the file")
            chunks.append(Chunk(cid, payload))
            pos += 8 + length
            if cid == CHUNK_EOF:
                break
        return cls(chunks, data[pos:])

    def to_bytes(self) -> bytes:
        out = bytearray()
        for c in self.chunks:
            if len(out) % 0x20:
                raise ModError(f"chunk 0x{c.id:x} would start off a 32-byte boundary")
            out += struct.pack(">II", c.id, len(c.payload)) + c.payload
        return bytes(out) + self.tail

    def get(self, cid: int) -> Chunk | None:
        return next((c for c in self.chunks if c.id == cid), None)

    def set(self, cid: int, payload: bytes | None) -> None:
        """Replace (or insert, keeping id order; or remove when `payload` is None) a chunk."""
        for i, c in enumerate(self.chunks):
            if c.id == cid:
                if payload is None:
                    del self.chunks[i]
                else:
                    c.payload = payload
                return
        if payload is None:
            return
        for i, c in enumerate(self.chunks):
            if c.id > cid:
                self.chunks.insert(i, Chunk(cid, payload))
                return
        raise ModError("no end-of-file chunk")


# ----------------------------------------------------------------------------- simple arrays


def _counted(payload: bytes):
    r = Reader(payload, base=8)
    n = r.u32()
    r.pad()
    return r, n


def _begin_counted(n: int) -> Writer:
    w = Writer()
    w.u32(n)
    w.pad()
    return w


def decode_vec3s(payload: bytes) -> list[tuple[float, float, float]]:
    r, n = _counted(payload)
    return [(r.f32(), r.f32(), r.f32()) for _ in range(n)]


def encode_vec3s(items) -> bytes:
    w = _begin_counted(len(items))
    for x, y, z in items:
        w.f32(x)
        w.f32(y)
        w.f32(z)
    w.pad()
    return bytes(w.buf)


def decode_vec2s(payload: bytes) -> list[tuple[float, float]]:
    r, n = _counted(payload)
    return [(r.f32(), r.f32()) for _ in range(n)]


def encode_vec2s(items) -> bytes:
    w = _begin_counted(len(items))
    for u, v in items:
        w.f32(u)
        w.f32(v)
    w.pad()
    return bytes(w.buf)


def decode_colours(payload: bytes) -> list[tuple[int, int, int, int]]:
    r, n = _counted(payload)
    return [(r.u8(), r.u8(), r.u8(), r.u8()) for _ in range(n)]


def encode_colours(items) -> bytes:
    w = _begin_counted(len(items))
    for c in items:
        w.raw(bytes(c))
    w.pad()
    return bytes(w.buf)


# ----------------------------------------------------------------------------- textures


@dataclass
class TexImg:
    width: int
    height: int
    fmt: int
    image_count: int
    unknown: tuple[int, int, int, int]
    data: bytes


def decode_textures(payload: bytes) -> list[TexImg]:
    r, n = _counted(payload)
    out = []
    for _ in range(n):
        w, h, fmt, count = r.u16(), r.u16(), r.u32(), r.u32()
        unknown = (r.u32(), r.u32(), r.u32(), r.u32())
        size = r.u32()
        out.append(TexImg(w, h, fmt, count, unknown, r.raw(size)))
    return out


def encode_textures(items: list[TexImg]) -> bytes:
    w = _begin_counted(len(items))
    for t in items:
        w.u16(t.width)
        w.u16(t.height)
        w.u32(t.fmt)
        w.u32(t.image_count)
        for u in t.unknown:
            w.u32(u)
        w.u32(len(t.data))
        w.raw(t.data)
    w.pad()
    return bytes(w.buf)


@dataclass
class TexAttr:
    texture_index: int
    unknown: int
    tiling: int
    use_offset_img: int
    lod_bias: float


def decode_texattrs(payload: bytes) -> list[TexAttr]:
    r, n = _counted(payload)
    return [TexAttr(r.s16(), r.s16(), r.s16(), r.s16(), r.f32()) for _ in range(n)]


def encode_texattrs(items: list[TexAttr]) -> bytes:
    w = _begin_counted(len(items))
    for a in items:
        w.s16(a.texture_index)
        w.s16(a.unknown)
        w.s16(a.tiling)
        w.s16(a.use_offset_img)
        w.f32(a.lod_bias)
    w.pad()
    return bytes(w.buf)


# ----------------------------------------------------------------------------- materials

KEY = 12  # every PVW keyframe value (u8/s10/f32 flavours) is 12 bytes
ANIMKEY1 = 4 + KEY
ANIMKEY3 = 4 + 3 * KEY


def _skip_anim1(r: Reader) -> None:
    n = r.u32()
    r.raw(n * ANIMKEY1)


def _skip_anim3(r: Reader) -> None:
    n = r.u32()
    r.raw(n * ANIMKEY3)


@dataclass
class Material:
    """A raw material record; `tex_attr_offsets` locate each PVWTextureData's texattr index."""

    raw: bytes
    flags: int
    tev_index: int | None
    tex_attr_offsets: list[int] = field(default_factory=list)

    def tex_attrs(self) -> list[int]:
        return [struct.unpack_from(">I", self.raw, o)[0] for o in self.tex_attr_offsets]

    def with_tex_attr(self, index: int, slot: int = 0) -> "Material":
        b = bytearray(self.raw)
        struct.pack_into(">I", b, self.tex_attr_offsets[slot], index)
        return Material(bytes(b), self.flags, self.tev_index, list(self.tex_attr_offsets))


@dataclass
class MaterialChunk:
    tev_infos: list[bytes]
    materials: list[Material]


def _read_tev_info(r: Reader) -> None:
    for _ in range(3):  # PVWTevColReg
        r.raw(8 + 4 + 4)
        _skip_anim3(r)
        _skip_anim1(r)
    r.raw(4 * 4)  # konst colours
    stages = r.u32()
    r.raw(stages * 32)


def _read_material(r: Reader) -> tuple[int, int | None, list[int]]:
    start = r.pos
    flags = r.u32()
    r.u32()  # texture index (non-PVW path)
    r.raw(4)  # colour
    tev = None
    offsets: list[int] = []
    if flags & MATFLAG_PVW:
        tev = r.u32()
        # PVWPolygonColourInfo
        r.raw(4 + 4 + 4)
        _skip_anim3(r)
        _skip_anim1(r)
        r.raw(8)  # PVWLightingInfo
        r.raw(16)  # PVWPeInfo
        # PVWTextureInfo
        r.raw(4 + 12)
        texgens = r.u32()
        r.raw(texgens * 4)
        tex_datas = r.u32()
        for _ in range(tex_datas):
            offsets.append(r.pos - start)
            r.raw(4 + 2 + 2 + 4 + 4 + 4 + 4 + 7 * 4)
            for _ in range(3):
                _skip_anim3(r)
    return flags, tev, offsets


def decode_materials(payload: bytes) -> MaterialChunk:
    r = Reader(payload, base=8)
    mat_count, tev_count = r.u32(), r.u32()
    r.pad()
    tevs = []
    for _ in range(tev_count):
        s = r.pos
        _read_tev_info(r)
        tevs.append(payload[s : r.pos])
    mats = []
    for _ in range(mat_count):
        s = r.pos
        flags, tev, offsets = _read_material(r)
        mats.append(Material(payload[s : r.pos], flags, tev, offsets))
    return MaterialChunk(tevs, mats)


def encode_materials(chunk: MaterialChunk) -> bytes:
    w = Writer()
    w.u32(len(chunk.materials))
    w.u32(len(chunk.tev_infos))
    w.pad()
    for t in chunk.tev_infos:
        w.raw(t)
    for m in chunk.materials:
        w.raw(m.raw)
    w.pad()
    return bytes(w.buf)


# ----------------------------------------------------------------------------- meshes


@dataclass
class DispList:
    flags: int
    face_count: int
    data: bytes


@dataclass
class MtxGroup:
    deps: list[int]
    displists: list[DispList]


@dataclass
class Mesh:
    parent_joint: int
    flags: int
    groups: list[MtxGroup]


def decode_meshes(payload: bytes) -> list[Mesh]:
    r, n = _counted(payload)
    meshes = []
    for _ in range(n):
        parent, flags, group_count = r.s32(), r.u32(), r.u32()
        groups = []
        for _ in range(group_count):
            dep_len = r.u32()
            deps = [r.s16() for _ in range(dep_len)]
            dl_count = r.u32()
            dls = []
            for _ in range(dl_count):
                f, faces, size = r.u32(), r.u32(), r.u32()
                r.pad()
                dls.append(DispList(f, faces, r.raw(size)))
            groups.append(MtxGroup(deps, dls))
        meshes.append(Mesh(parent, flags, groups))
    return meshes


def encode_meshes(meshes: list[Mesh]) -> bytes:
    w = _begin_counted(len(meshes))
    for m in meshes:
        w.s32(m.parent_joint)
        w.u32(m.flags)
        w.u32(len(m.groups))
        for g in m.groups:
            w.u32(len(g.deps))
            for d in g.deps:
                w.s16(d)
            w.u32(len(g.displists))
            for dl in g.displists:
                w.u32(dl.flags)
                w.u32(dl.face_count)
                w.u32(len(dl.data))
                w.pad()
                w.raw(dl.data)
    w.pad()
    return bytes(w.buf)


# ----------------------------------------------------------------------------- joints


@dataclass
class Joint:
    parent: int
    flags: int
    bounds_min: tuple[float, float, float]
    bounds_max: tuple[float, float, float]
    unknown: float
    scale: tuple[float, float, float]
    rotation: tuple[float, float, float]
    translation: tuple[float, float, float]
    matpolys: list[tuple[int, int]]  # (material index, mesh index)


def _v3(r: Reader):
    return (r.f32(), r.f32(), r.f32())


def decode_joints(payload: bytes) -> list[Joint]:
    r, n = _counted(payload)
    out = []
    for _ in range(n):
        parent, flags = r.s32(), r.u32()
        bmin, bmax, unk = _v3(r), _v3(r), r.f32()
        scale, rot, trans = _v3(r), _v3(r), _v3(r)
        count = r.u32()
        polys = [(r.s16(), r.s16()) for _ in range(count)]
        out.append(Joint(parent, flags, bmin, bmax, unk, scale, rot, trans, polys))
    return out


def encode_joints(joints: list[Joint]) -> bytes:
    w = _begin_counted(len(joints))
    for j in joints:
        w.s32(j.parent)
        w.u32(j.flags)
        for v in (*j.bounds_min, *j.bounds_max, j.unknown, *j.scale, *j.rotation, *j.translation):
            w.f32(v)
        w.u32(len(j.matpolys))
        for mat, mesh in j.matpolys:
            w.s16(mat)
            w.s16(mesh)
    w.pad()
    return bytes(w.buf)


# ----------------------------------------------------------------------------- collision


@dataclass
class CollTri:
    map_code: int
    verts: tuple[int, int, int]
    room: int
    adjacent: tuple[int, int, int]
    plane: tuple[float, float, float, float]  # normal xyz, offset

    @property
    def attribute(self) -> int:
        return self.map_code >> 29


@dataclass
class CollPrism:
    rooms: list[int]
    tris: list[CollTri]


def decode_collprism(payload: bytes) -> CollPrism:
    r = Reader(payload, base=8)
    tri_count, room_count = r.u32(), r.u32()
    r.pad()
    rooms = [r.s32() for _ in range(room_count)]
    r.pad()
    tris = []
    for _ in range(tri_count):
        code = r.u32()
        verts = (r.u32(), r.u32(), r.u32())
        room = r.s16()
        adj = (r.s16(), r.s16(), r.s16())
        plane = (r.f32(), r.f32(), r.f32(), r.f32())
        tris.append(CollTri(code, verts, room, adj, plane))
    return CollPrism(rooms, tris)


def encode_collprism(c: CollPrism) -> bytes:
    w = Writer()
    w.u32(len(c.tris))
    w.u32(len(c.rooms))
    w.pad()
    for room in c.rooms:
        w.s32(room)
    w.pad()
    for t in c.tris:
        w.u32(t.map_code)
        for v in t.verts:
            w.u32(v)
        w.s16(t.room)
        for a in t.adjacent:
            w.s16(a)
        for p in t.plane:
            w.f32(p)
    w.pad()
    return bytes(w.buf)


# ----------------------------------------------------------------------------- display lists


def decode_displist(data: bytes, mesh_flags: int) -> list[list[tuple]]:
    """Decode a mesh display list into triangle-primitive vertex tuples.

    Each vertex is (mtx_idx or None, pos, nrm, clr or None, (tex0..)).
    """
    tex_count = sum(1 for i in range(8) if mesh_flags & (MESH_TEX0 << i))
    prims = []
    p = 0
    while p < len(data):
        op = data[p]
        if op == 0:  # GX_NOP padding
            p += 1
            continue
        count = struct.unpack_from(">H", data, p + 1)[0]
        p += 3
        verts = []
        for _ in range(count):
            mtx = None
            if mesh_flags & MESH_POS_NRM:
                mtx = data[p]
                p += 1
            if mesh_flags & MESH_TEX1MTX:
                p += 1
            pos, nrm = struct.unpack_from(">HH", data, p)
            p += 4
            clr = None
            if mesh_flags & MESH_VTXCOLOR:
                clr = struct.unpack_from(">H", data, p)[0]
                p += 2
            tex = struct.unpack_from(f">{tex_count}H", data, p) if tex_count else ()
            p += 2 * tex_count
            verts.append((mtx, pos, nrm, clr, tex))
        prims.append((op & 0xF8, verts))
    return prims


def encode_triangles(verts: list[tuple], mesh_flags: int) -> bytes:
    """Encode a GX_TRIANGLES display list from vertex tuples (see `decode_displist`)."""
    out = bytearray()
    # A single draw command holds at most 65535 vertices; split on whole triangles.
    for start in range(0, len(verts), 65535 - 65535 % 3):
        chunk = verts[start : start + 65535 - 65535 % 3]
        out += struct.pack(">BH", GX_TRIANGLES, len(chunk))
        for mtx, pos, nrm, clr, tex in chunk:
            if mesh_flags & MESH_POS_NRM:
                out.append(mtx or 0)
            if mesh_flags & MESH_TEX1MTX:
                out.append(0)
            out += struct.pack(">HH", pos, nrm)
            if mesh_flags & MESH_VTXCOLOR:
                out += struct.pack(">H", clr or 0)
            for t in tex:
                out += struct.pack(">H", t)
    # Display lists are fetched in 32-byte units; pad with GX_NOPs.
    out += b"\0" * ((32 - len(out) % 32) % 32)
    return bytes(out)
