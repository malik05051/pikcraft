"""Read and patch GameCube disc images (ISO/GCM).

The patched image is a byte-for-byte copy of the original with replacement files appended at the
end and their file-system (FST) entries repointed, so nothing else on the disc moves.
"""

from __future__ import annotations

import hashlib
import os
import shutil
import struct
from dataclasses import dataclass

MAGIC_OFFSET = 0x1C
GC_MAGIC = 0xC2339F3D
DOL_OFFSET_FIELD = 0x420
FST_OFFSET_FIELD = 0x424
FST_SIZE_FIELD = 0x428
APPEND_ALIGN = 0x8000


class DiscError(Exception):
    pass


@dataclass
class FstFile:
    path: str
    entry_index: int
    offset: int
    size: int


def _u32(buf: bytes, off: int) -> int:
    return struct.unpack_from(">I", buf, off)[0]


class GameCubeDisc:
    def __init__(self, path: str):
        self.path = path
        self._f = open(path, "rb")
        header = self._f.read(0x440)
        if len(header) < 0x440 or _u32(header, MAGIC_OFFSET) != GC_MAGIC:
            self._f.close()
            raise DiscError(
                f"{path} is not a GameCube disc image. Use a plain .iso/.gcm "
                "(in Dolphin: right-click the game > Convert File > ISO)."
            )
        self.header = header
        self.game_id = header[0:6].decode("ascii", "replace")
        self.revision = header[7]
        self.title = header[0x20:0x60].split(b"\0", 1)[0].decode("ascii", "replace")
        self.dol_offset = _u32(header, DOL_OFFSET_FIELD)
        self.fst_offset = _u32(header, FST_OFFSET_FIELD)
        self.fst_size = _u32(header, FST_SIZE_FIELD)
        self._f.seek(self.fst_offset)
        self.fst = self._f.read(self.fst_size)
        self.files: dict[str, FstFile] = {}
        self._parse_fst()

    @property
    def version_tag(self) -> str:
        """e.g. GPIE01_01, matching the decompilation's version names."""
        return f"{self.game_id}_{self.revision:02d}"

    def close(self) -> None:
        self._f.close()

    def __enter__(self) -> "GameCubeDisc":
        return self

    def __exit__(self, *exc) -> None:
        self.close()

    def _parse_fst(self) -> None:
        fst = self.fst
        count = _u32(fst, 8)
        strings = count * 12

        def name_at(i: int) -> str:
            off = _u32(fst, i * 12) & 0xFFFFFF
            end = fst.index(b"\0", strings + off)
            return fst[strings + off : end].decode("shift_jis", "replace")

        # Walk directories: each dir entry stores the index one past its last child.
        stack: list[tuple[int, str]] = [(count, "")]
        i = 1
        while i < count:
            while stack and i >= stack[-1][0]:
                stack.pop()
            prefix = stack[-1][1] if stack else ""
            word0 = _u32(fst, i * 12)
            is_dir = (word0 >> 24) != 0
            name = name_at(i)
            path = f"{prefix}{name}"
            if is_dir:
                next_index = _u32(fst, i * 12 + 8)
                stack.append((next_index, path + "/"))
            else:
                self.files[path.lower()] = FstFile(path, i, _u32(fst, i * 12 + 4), _u32(fst, i * 12 + 8))
            i += 1

    def find(self, path: str) -> FstFile:
        f = self.files.get(path.lower().lstrip("/"))
        if not f:
            raise DiscError(f"{path} is not on the disc")
        return f

    def read(self, path: str) -> bytes:
        f = self.find(path)
        self._f.seek(f.offset)
        return self._f.read(f.size)

    def read_dol(self) -> bytes:
        self._f.seek(self.dol_offset)
        head = self._f.read(0x100)
        size = 0x100
        for k in range(18):
            off, length = _u32(head, k * 4), _u32(head, 0x90 + k * 4)
            if length:
                size = max(size, off + length)
        self._f.seek(self.dol_offset)
        return self._f.read(size)

    def sha1(self) -> str:
        h = hashlib.sha1()
        self._f.seek(0)
        while chunk := self._f.read(1 << 20):
            h.update(chunk)
        return h.hexdigest()


def write_patched(
    src_path: str,
    dst_path: str,
    replacements: dict[str, bytes],
    game_id: str | None = None,
    dol: bytes | None = None,
) -> None:
    """Copy `src_path` to `dst_path`, replacing the given disc files (paths as on the disc) and,
    optionally, the main executable."""
    with GameCubeDisc(src_path) as disc:
        entries = [(disc.find(p), data) for p, data in replacements.items()]
        fst_offset = disc.fst_offset
        fst = bytearray(disc.fst)
        dol_offset = disc.dol_offset
        old_dol_size = len(disc.read_dol())
    if os.path.abspath(src_path) == os.path.abspath(dst_path):
        raise DiscError("refusing to overwrite the original disc image")
    shutil.copyfile(src_path, dst_path)
    with open(dst_path, "r+b") as out:
        out.seek(0, os.SEEK_END)
        end = out.tell()
        for entry, data in entries:
            if len(data) <= entry.size:
                # Fits in place: reuse the original slot.
                out.seek(entry.offset)
                out.write(data)
                new_offset = entry.offset
            else:
                end = (end + APPEND_ALIGN - 1) // APPEND_ALIGN * APPEND_ALIGN
                out.seek(end)
                out.write(data)
                new_offset = end
                end += len(data)
            struct.pack_into(">II", fst, entry.entry_index * 12 + 4, new_offset, len(data))
        if dol is not None:
            if len(dol) <= old_dol_size:
                out.seek(dol_offset)
                out.write(dol)
            else:
                end = (end + APPEND_ALIGN - 1) // APPEND_ALIGN * APPEND_ALIGN
                out.seek(end)
                out.write(dol)
                out.seek(DOL_OFFSET_FIELD)
                out.write(struct.pack(">I", end))
                end += len(dol)
        # Pad the image to a whole sector so emulators see a tidy size.
        out.seek(0, os.SEEK_END)
        size = out.tell()
        if size % APPEND_ALIGN:
            out.write(b"\0" * (APPEND_ALIGN - size % APPEND_ALIGN))
        out.seek(fst_offset)
        out.write(fst)
        if game_id:
            out.seek(0)
            out.write(game_id.encode("ascii")[:6])
