"""Original block textures (procedural pixel art, no Mojang assets) and GameCube texture encoding."""

from __future__ import annotations

import struct

from .modfile import TexImg

TILE = 16

# Pikmin's TexImgFormat values (include/Texture.h)
FMT_RGB565 = 0
FMT_RGB5A3 = 2


def _hash(seed: int, x: int, y: int, z: int) -> int:
    h = (seed ^ (x * 0x8DA6B343) ^ (y * 0xD8163841) ^ (z * 0xCB1AB31F)) & 0xFFFFFFFF
    h ^= h >> 13
    h = (h * 0x5BD1E995) & 0xFFFFFFFF
    h ^= h >> 15
    return h


def _noise(seed: int, t: int, x: int, y: int) -> float:
    return _hash(seed ^ (t * 7717), x, y, 3) / 0xFFFFFFFF


def _shade(c, k: float, a: int = 255):
    return tuple(max(0, min(255, int(v * k * 255))) for v in c) + (a,)


GRASS = (0.36, 0.66, 0.26)
DIRT = (0.53, 0.37, 0.24)
STONE = (0.52, 0.52, 0.54)
SAND = (0.86, 0.80, 0.56)
WOOD = (0.45, 0.32, 0.18)
PLANK = (0.74, 0.58, 0.35)
LEAF = (0.22, 0.52, 0.18)
WATER = (0.22, 0.42, 0.86)
MUD = (0.40, 0.30, 0.20)


def _pixel(name: str, x: int, y: int):
    t = sum(map(ord, name))
    n = _noise(1, t, x, y)
    n2 = _noise(2, t, x // 2, y // 2)
    grain = 0.86 + n * 0.18
    if name == "grass_top":
        return _shade(GRASS, 0.8 + n * 0.3)
    if name == "grass_side":
        edge = 3 + int(_noise(5, t, x, 0) * 3)
        return _shade(GRASS, 0.8 + n * 0.3) if y < edge else _shade(DIRT, grain)
    if name == "dirt":
        return _shade(DIRT, grain)
    if name == "mud":
        return _shade(MUD, 0.8 + n2 * 0.3)
    if name == "stone":
        return _shade(STONE, 0.82 + n * 0.12 + n2 * 0.12)
    if name == "sand":
        return _shade(SAND, 0.92 + n * 0.1)
    if name == "log":
        stripe = 0.75 if (x + int(n2 * 2)) % 4 == 0 else 1.0
        return _shade(WOOD, stripe * (0.9 + n * 0.12))
    if name == "planks":
        seam = y % 4 == 3 or (x + (y // 4) * 5) % 16 == 0
        return _shade(PLANK, 0.7 if seam else 0.92 + n * 0.1)
    if name == "leaves":
        if n < 0.18:
            return (0, 0, 0, 0)
        return _shade(LEAF, 0.75 + n * 0.4)
    if name == "water":
        return _shade(WATER, 0.85 + n2 * 0.2, 170)
    raise KeyError(name)


BLOCK_TEXTURES = ["grass_top", "grass_side", "dirt", "stone", "sand", "log", "planks", "leaves", "water", "mud"]
TRANSLUCENT = {"leaves", "water"}


def pixels(name: str, size: int = TILE) -> list[tuple[int, int, int, int]]:
    """RGBA pixels, row-major. Larger sizes repeat each texel (nearest-neighbour upscale)."""
    k = size // TILE
    return [_pixel(name, x // k, y // k) for y in range(size) for x in range(size)]


def _rgb565(r, g, b, a=255) -> int:
    return ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3)


def _rgb5a3(r, g, b, a) -> int:
    if a >= 0xE0:
        return 0x8000 | ((r >> 3) << 10) | ((g >> 3) << 5) | (b >> 3)
    return ((a >> 5) << 12) | ((r >> 4) << 8) | ((g >> 4) << 4) | (b >> 4)


def encode(rgba: list[tuple[int, int, int, int]], width: int, height: int, fmt: int) -> bytes:
    """Encode to a GameCube 16-bit texture format (4x4 pixel tiles, big-endian)."""
    conv = {FMT_RGB565: _rgb565, FMT_RGB5A3: _rgb5a3}[fmt]
    out = bytearray()
    for ty in range(0, height, 4):
        for tx in range(0, width, 4):
            for y in range(ty, ty + 4):
                for x in range(tx, tx + 4):
                    out += struct.pack(">H", conv(*rgba[y * width + x]))
    return bytes(out)


def decode(data: bytes, width: int, height: int, fmt: int) -> list[tuple[int, int, int, int]]:
    """Inverse of `encode` (used by tests and previews)."""
    px = [(0, 0, 0, 0)] * (width * height)
    i = 0
    for ty in range(0, height, 4):
        for tx in range(0, width, 4):
            for y in range(ty, ty + 4):
                for x in range(tx, tx + 4):
                    v = struct.unpack_from(">H", data, i)[0]
                    i += 2
                    if fmt == FMT_RGB565:
                        c = ((v >> 11) << 3, ((v >> 5) & 63) << 2, (v & 31) << 3, 255)
                    elif v & 0x8000:
                        c = (((v >> 10) & 31) << 3, ((v >> 5) & 31) << 3, (v & 31) << 3, 255)
                    else:
                        c = (((v >> 8) & 15) * 17, ((v >> 4) & 15) * 17, (v & 15) * 17, ((v >> 12) & 7) * 36)
                    px[y * width + x] = c
    return px


def make_teximg(name: str, size: int = 32) -> TexImg:
    """A block texture as a .mod texture entry. 32x32 keeps pixels crisp even with filtering."""
    fmt = FMT_RGB5A3 if name in TRANSLUCENT else FMT_RGB565
    return TexImg(size, size, fmt, 1, (0, 0, 0, 0), encode(pixels(name, size), size, size, fmt))
