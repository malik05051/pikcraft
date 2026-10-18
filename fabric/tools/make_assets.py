#!/usr/bin/env python3
"""Draws every texture and synthesizes every sound of the PikCraft mod.

All of it is generated here from code, so the mod ships no art or audio from anyone else.
Run from the fabric/ folder: python3 tools/make_assets.py (needs Pillow, numpy and oggenc).
"""
import json
import math
import os
import random
import subprocess
import tempfile
import wave

import numpy as np
from PIL import Image, ImageDraw

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "main", "resources", "assets")
ASSETS = os.path.join(ROOT, "pikcraft")
TEX = os.path.join(ASSETS, "textures")

TYPES = {
    "red": (232, 48, 42),
    "yellow": (242, 215, 44),
    "blue": (47, 111, 232),
    "rock": (110, 106, 122),
    "winged": (240, 140, 200),
    "purple": (122, 63, 184),
    "white": (242, 242, 242),
}

CLEAR = (0, 0, 0, 0)


def save(img, *path):
    full = os.path.join(*path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    img.save(full)


def shade(colour, factor):
    return tuple(max(0, min(255, int(c * factor))) for c in colour[:3]) + ((colour[3],) if len(colour) == 4 else (255,))


def mix(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3)) + (255,)


def box_faces(u, v, w, h, d):
    """Where Minecraft's cube UV layout puts each face: (x, y, width, height)."""
    return {
        "top": (u + d, v, w, d),
        "bottom": (u + d + w, v, w, d),
        "east": (u, v + d, d, h),
        "front": (u + d, v + d, w, h),
        "west": (u + d + w, v + d, d, h),
        "back": (u + 2 * d + w, v + d, w, h),
    }


def fill_rect(img, rect, colour_fn):
    x0, y0, w, h = rect
    for y in range(h):
        for x in range(w):
            c = colour_fn(x, y, w, h)
            if c is not None:
                img.putpixel((x0 + x, y0 + y), c)


def paint_box(img, u, v, w, h, d, colour, rng, noise=0.08, faces=None):
    """A box in one colour, lit from above, with a little pixel noise."""
    light = {"top": 1.12, "bottom": 0.7, "front": 1.0, "back": 0.9, "east": 0.88, "west": 0.88}
    for name, rect in box_faces(u, v, w, h, d).items():
        if faces and name not in faces:
            continue
        base = light[name]

        def fn(x, y, fw, fh, base=base, name=name):
            gradient = 1.0 if name in ("top", "bottom") else 1.05 - 0.15 * (y / max(1, fh - 1))
            return shade(colour, base * gradient * (1.0 + rng.uniform(-noise, noise)))

        fill_rect(img, rect, fn)


# ---------------------------------------------------------------------------------------- Pikmin

def leaf_shape(x, y, w, h):
    """A pointed leaf in a w x h face, tip at the top."""
    cx = (w - 1) / 2
    half = (w / 2) * math.sin(math.pi * (y + 0.5) / h) * 1.05
    return abs(x - cx) <= half


def pikmin_texture(name, colour, rng):
    img = Image.new("RGBA", (64, 64), CLEAR)
    body = colour
    if name == "rock":
        noise = 0.16
    else:
        noise = 0.06
    paint_box(img, 0, 0, 6, 6, 6, body, rng, noise)          # head
    paint_box(img, 0, 12, 3, 5, 2, body, rng, noise)         # body
    paint_box(img, 24, 0, 1, 4, 1, shade(body, 0.92), rng, noise)  # arms
    paint_box(img, 28, 0, 1, 4, 1, shade(body, 0.92), rng, noise)
    leg = shade(body, 0.8)
    paint_box(img, 24, 6, 1, 3, 1, leg, rng, noise)
    paint_box(img, 28, 6, 1, 3, 1, leg, rng, noise)
    paint_box(img, 32, 0, 1, 1, 2, shade(body, 1.05), rng, noise)  # red's nose
    paint_box(img, 24, 12, 1, 3, 2, shade(body, 0.95), rng, noise)  # yellow's ears
    paint_box(img, 30, 12, 1, 3, 2, shade(body, 0.95), rng, noise)

    # face: big Minecraft-style eyes
    fx, fy = 6, 6
    sclera = (250, 250, 250, 255)
    pupil = (200, 20, 30, 255) if name == "white" else (25, 25, 30, 255)
    if name == "rock":
        for (ex, ey) in ((1, 2), (4, 2)):
            img.putpixel((fx + ex, fy + ey), (20, 20, 24, 255))
            img.putpixel((fx + ex, fy + ey + 1), (60, 58, 66, 255))
        for _ in range(6):  # cracks
            x, y = rng.randrange(6), rng.randrange(6)
            img.putpixel((fx + x, fy + y), shade(body, 0.6))
    else:
        for (sx, px) in ((1, 2), (4, 3)):
            img.putpixel((fx + sx, fy + 2), sclera)
            img.putpixel((fx + sx, fy + 3), sclera)
            img.putpixel((fx + px, fy + 2), sclera)
            img.putpixel((fx + px, fy + 3), pupil)
    if name == "blue":
        img.putpixel((fx + 2, fy + 4), shade(body, 0.55))
        img.putpixel((fx + 3, fy + 4), shade(body, 0.55))
    if name == "white":
        for x in (1, 4):
            img.putpixel((fx + x, fy + 1), (255, 255, 255, 255))
    # back of the head: a slightly darker patch
    bx, by, bw, bh = box_faces(0, 0, 6, 6, 6)["back"]
    for x in range(1, 5):
        img.putpixel((bx + x, by + 4), shade(body, 0.85))
    # the nose tip
    nx, ny, _, _ = box_faces(32, 0, 1, 1, 2)["front"]
    img.putpixel((nx, ny), shade(body, 0.8))

    # purple's hair: a few dark strands
    for face_x in (24, 28):
        for x in range(4):
            for y in range(2):
                if (x + y) % 2 == 0 or y == 1:
                    img.putpixel((face_x + x, 20 + y), shade(body, 0.55 + 0.1 * rng.random()))

    # winged's wings: pale and veined
    for (u, v) in ((0, 24), (12, 24)):
        for rect in (box_faces(u, v, 0, 4, 5)["east"], box_faces(u, v, 0, 4, 5)["west"]):
            def wing(x, y, w, h):
                if (x == 0 and y == h - 1) or (x == w - 1 and y == 0):
                    return None
                vein = x == y or y == 1
                return (235, 200, 225, 255) if vein else (252, 238, 248, 255)
            fill_rect(img, rect, wing)

    # stem, leaf, bud and flower
    stem = (96, 150, 58)
    paint_box(img, 32, 4, 1, 6, 1, stem, rng, 0.05)
    leaf = (74, 170, 60)
    for rect in (box_faces(40, 0, 0, 4, 4)["east"], box_faces(40, 0, 0, 4, 4)["west"], box_faces(48, 0, 4, 4, 0)["front"],
                 box_faces(48, 0, 4, 4, 0)["back"]):
        fill_rect(img, rect, lambda x, y, w, h: (shade(leaf, 1.15 if x == (w // 2) else 1.0) if leaf_shape(x, y, w, h) else None))
    petal = {"purple": (246, 160, 205), "white": (196, 132, 232)}.get(name, (250, 250, 246))
    bud = {"purple": (238, 150, 196), "white": (190, 130, 226)}.get(name, (246, 236, 214))
    paint_box(img, 48, 4, 2, 2, 2, bud, rng, 0.04)
    for rect in (box_faces(40, 10, 0, 5, 5)["east"], box_faces(40, 10, 0, 5, 5)["west"], box_faces(50, 10, 5, 5, 0)["front"],
                 box_faces(50, 10, 5, 5, 0)["back"]):
        def flower(x, y, w, h):
            dx, dy = x - 2, y - 2
            if dx == 0 and dy == 0:
                return (250, 210, 40, 255)
            if abs(dx) + abs(dy) <= 2 or (abs(dx) == 2 and abs(dy) == 0) or (abs(dy) == 2 and abs(dx) == 0):
                return petal + (255,)
            return None
        fill_rect(img, rect, flower)
    return img


def pikmin_icon(colour, name):
    """16 x 16 HUD icon: a Pikmin head with its leaf."""
    img = Image.new("RGBA", (16, 16), CLEAR)
    d = ImageDraw.Draw(img)
    d.rectangle([3, 6, 12, 15], fill=shade(colour, 1.0), outline=shade(colour, 0.6))
    if name != "rock":
        d.line([8, 6, 8, 2], fill=(96, 150, 58, 255))
        d.polygon([(8, 0), (11, 2), (8, 4), (5, 2)], fill=(74, 170, 60, 255))
    eye = (200, 20, 30, 255) if name == "white" else (20, 20, 24, 255)
    for x in (5, 9):
        d.rectangle([x, 9, x + 1, 11], fill=(250, 250, 250, 255))
    img.putpixel((6, 10), eye)
    img.putpixel((9, 10), eye)
    if name == "red":
        d.rectangle([7, 11, 8, 13], fill=shade(colour, 1.1))
    if name == "yellow":
        d.rectangle([1, 7, 2, 10], fill=shade(colour, 0.9))
        d.rectangle([13, 7, 14, 10], fill=shade(colour, 0.9))
    if name == "winged":
        d.rectangle([0, 9, 2, 12], fill=(252, 238, 248, 255))
        d.rectangle([13, 9, 15, 12], fill=(252, 238, 248, 255))
    if name == "purple":
        d.line([5, 5, 6, 3], fill=shade(colour, 0.55))
        d.line([10, 5, 11, 3], fill=shade(colour, 0.55))
    return img


# ---------------------------------------------------------------------------------------- Onion

def onion_texture(name, colour, rng):
    img = Image.new("RGBA", (128, 128), CLEAR)
    dark = shade(colour, 0.7)
    light = shade(colour, 1.25)
    faces = box_faces(0, 0, 24, 14, 24)
    for face, rect in faces.items():
        def bulb(x, y, w, h, face=face):
            if face in ("top", "bottom"):
                # rings around the middle, like the bulb's ribs
                r = math.hypot(x - w / 2 + 0.5, y - h / 2 + 0.5)
                c = mix(light, colour, min(1.0, r / 12)) if face == "top" else dark
                if int(r) % 4 == 0:
                    c = shade(c, 0.9)
                return shade(c, 1.0 + rng.uniform(-0.04, 0.04))
            t = y / (h - 1)
            c = mix(light, dark, t)
            if y == 6 or y == 7:
                c = (250, 250, 240, 255) if (x % 4 in (1, 2)) else shade(colour, 1.4)  # glowing band
            if x % 6 == 0 and y not in (6, 7):
                c = shade(c, 0.9)  # ribs
            return shade(c, 1.0 + rng.uniform(-0.03, 0.03))
        fill_rect(img, rect, bulb)
    paint_box(img, 0, 40, 16, 4, 16, dark, rng, 0.05)  # underside
    paint_box(img, 0, 60, 16, 4, 16, light, rng, 0.05)  # cap
    paint_box(img, 64, 40, 2, 8, 2, (214, 196, 150), rng, 0.05)  # stalk
    leg = shade(colour, 0.8)
    for face, rect in box_faces(72, 40, 2, 32, 2).items():
        fill_rect(img, rect, lambda x, y, w, h: shade(leg, 0.7 if y % 8 == 7 else 1.0 + rng.uniform(-0.04, 0.04)))
    # petals: a flower seen from above, five petals in the Onion's colour around a white centre
    for face in ("top", "bottom"):
        x0, y0, w, h = box_faces(80, 80, 16, 0, 16)[face]
        for y in range(h):
            for x in range(w):
                dx, dy = x - 7.5, y - 7.5
                r = math.hypot(dx, dy)
                a = math.atan2(dy, dx)
                petal_edge = 4.5 + 3.3 * abs(math.cos(a * 2.5))
                if r < 2.5:
                    img.putpixel((x0 + x, y0 + y), (252, 248, 230, 255))
                elif r < petal_edge:
                    img.putpixel((x0 + x, y0 + y), mix(light, (255, 255, 255), 0.35 * (1 - r / 8)))
    return img


# ---------------------------------------------------------------------------------------- the rest

DIGITS = {
    "0": ["111", "101", "101", "101", "111"],
    "1": ["010", "110", "010", "010", "111"],
    "2": ["111", "001", "111", "100", "111"],
    "5": ["111", "100", "111", "001", "111"],
}


def draw_number(img, text, x0, y0, w, h, colour):
    width = len(text) * 4 - 1
    sx = x0 + (w - width) // 2
    sy = y0 + (h - 5) // 2
    for i, ch in enumerate(text):
        for y, row in enumerate(DIGITS[ch]):
            for x, bit in enumerate(row):
                if bit == "1":
                    img.putpixel((sx + i * 4 + x, sy + y), colour)


def pellet_texture(value, rng):
    img = Image.new("RGBA", (32, 32), CLEAR)
    base = (236, 236, 236)
    paint_box(img, 0, 0, 8, 3, 8, base, rng, 0.03)
    paint_box(img, 0, 20, 8, 8, 3, base, rng, 0.03)
    ink = (70, 70, 70, 255)
    x, y, w, h = box_faces(0, 0, 8, 3, 8)["top"]
    draw_number(img, str(value), x, y, w, h, ink)
    for face in ("front", "back"):
        x, y, w, h = box_faces(0, 20, 8, 8, 3)[face]
        draw_number(img, str(value), x, y, w, h, ink)
    return img


def seed_texture(rng):
    img = Image.new("RGBA", (16, 16), CLEAR)
    paint_box(img, 0, 0, 2, 3, 2, (240, 236, 220), rng, 0.04)
    for face in ("east", "west"):
        fill_rect(img, box_faces(8, 0, 0, 3, 3)[face], lambda x, y, w, h: (200, 240, 190, 255) if leaf_shape(x, y, w, h) else None)
    return img


def posy_texture(rng):
    img = Image.new("RGBA", (64, 64), CLEAR)
    paint_box(img, 0, 0, 2, 18, 2, (88, 150, 60), rng, 0.05)
    for (u, v) in ((8, 0), (8, 4)):
        for face in ("top", "bottom"):
            def leaf(x, y, w, h):
                half = (h / 2) * math.sin(math.pi * (x + 0.5) / w) * 1.05
                return shade((70, 165, 62), 1.1 if y == h // 2 else 1.0) if abs(y - (h - 1) / 2) <= half else None
            fill_rect(img, box_faces(u, v, 7, 0, 4)[face], leaf)

    def petals(x, y, w, h):
        dx, dy = x - 6.5, y - 6.5
        r = math.hypot(dx, dy)
        a = math.atan2(dy, dx)
        edge = 4.5 + 2.6 * abs(math.cos(a * 3))
        if r < 3.2:
            return (120, 92, 50, 255)
        if r < edge:
            return mix((255, 244, 180), (246, 196, 74), r / 7.5)
        return None
    for face in ("front", "back"):
        fill_rect(img, box_faces(0, 32, 14, 14, 0)[face], petals)
    for face in ("east", "west"):
        fill_rect(img, box_faces(28, 32, 0, 14, 14)[face], petals)
    return img


def nectar_egg_texture(rng):
    img = Image.new("RGBA", (64, 32), CLEAR)
    shell = (246, 226, 204)
    paint_box(img, 0, 0, 8, 10, 8, shell, rng, 0.03)
    paint_box(img, 0, 18, 6, 2, 6, shade(shell, 1.03), rng, 0.03)
    paint_box(img, 24, 18, 6, 1, 6, shade(shell, 0.9), rng, 0.03)
    for _ in range(70):
        x, y = rng.randrange(64), rng.randrange(32)
        if img.getpixel((x, y))[3]:
            img.putpixel((x, y), rng.choice([(240, 150, 120, 255), (236, 120, 160, 255), (250, 190, 90, 255)]))
    return img


def nectar_texture(rng):
    img = Image.new("RGBA", (64, 32), CLEAR)
    gold = (255, 200, 40)
    for u, v, w, h, d in ((0, 0, 12, 1, 12), (0, 13, 6, 1, 6)):
        for face, rect in box_faces(u, v, w, h, d).items():
            def fn(x, y, fw, fh):
                c = shade(gold, 1.0 + rng.uniform(-0.08, 0.1))
                if face == "top" and rng.random() < 0.08:
                    c = (255, 246, 200, 255)
                return c[:3] + (215,)
            fill_rect(img, rect, fn)
    return img


def health_wheel():
    frames = 21
    size = 24
    img = Image.new("RGBA", (frames * size, size), CLEAR)
    for i in range(frames):
        f = i / (frames - 1)
        colour = (88, 220, 80) if f > 0.5 else (240, 208, 40) if f > 0.25 else (232, 60, 48)
        for y in range(size):
            for x in range(size):
                dx, dy = x - 11.5, y - 11.5
                r = math.hypot(dx, dy)
                if r > 11.5:
                    continue
                if r > 10.3:
                    c = (16, 16, 20, 255)
                else:
                    # clockwise from the top
                    a = (math.degrees(math.atan2(dx, -dy)) + 360) % 360
                    filled = a <= f * 360 and f > 0
                    c = shade(colour, 1.15 - 0.3 * (r / 10.3)) if filled else (42, 42, 52, 220)
                    if r < 3.2:
                        c = (250, 250, 250, 255) if f > 0 else (90, 90, 90, 255)
                img.putpixel((i * size + x, y), c)
    return img


def day_meter():
    img = Image.new("RGBA", (128, 32), CLEAR)
    d = ImageDraw.Draw(img)
    for x in range(102):
        t = x / 101
        sky = mix((120, 196, 255), (255, 150, 70), max(0.0, t - 0.55) / 0.45)
        d.line([x, 1, x, 7], fill=sky)
    d.rectangle([0, 0, 101, 8], outline=(30, 30, 40, 255))
    for x in range(102):
        d.line([x, 11, x, 17], fill=(30, 40, 90, 255))
    d.rectangle([0, 10, 101, 18], outline=(14, 14, 24, 255))
    # sun and moon
    for (ox, sun) in ((0, True), (16, False)):
        for y in range(12):
            for x in range(12):
                r = math.hypot(x - 5.5, y - 5.5)
                if sun and r <= 5.8:
                    img.putpixel((ox + x, 20 + y), (255, 222, 60, 255) if r < 4.2 else (255, 170, 40, 255))
                if not sun and r <= 5.2 and math.hypot(x - 8, y - 4) > 4.2:
                    img.putpixel((ox + x, 20 + y), (230, 236, 255, 255))
    return img


def rounded(d, box, radius, fill, outline=None):
    d.rounded_rectangle(box, radius=radius, fill=fill, outline=outline)


def onion_menu():
    img = Image.new("RGBA", (256, 128), CLEAR)
    d = ImageDraw.Draw(img)
    rounded(d, [0, 0, 219, 119], 10, (16, 34, 40, 228), (120, 220, 200, 255))
    rounded(d, [3, 3, 216, 116], 8, None, (40, 90, 90, 255))
    rounded(d, [12, 24, 92, 66], 6, (8, 20, 24, 200))
    rounded(d, [127, 24, 207, 66], 6, (8, 20, 24, 200))
    # Onion icon (tinted by type in game): bulb on legs
    ox, oy = 224, 0
    d.ellipse([ox + 6, oy + 4, ox + 26, oy + 18], fill=(240, 240, 240, 255))
    d.rectangle([ox + 12, oy + 1, ox + 20, oy + 5], fill=(255, 255, 255, 255))
    for lx in (8, 15, 23):
        d.line([ox + 16, oy + 16, ox + lx, oy + 30], fill=(200, 200, 200, 255), width=2)
    return img


def inventory_background():
    """A Pikmin 4 style inventory: soft panel, round-cornered cells; same slot positions as vanilla."""
    img = Image.new("RGBA", (256, 256), CLEAR)
    d = ImageDraw.Draw(img)
    rounded(d, [0, 0, 175, 165], 7, (226, 236, 222, 255), (64, 96, 80, 255))
    rounded(d, [2, 2, 173, 163], 6, None, (250, 255, 248, 255))
    # leafy dots in the background
    rng = random.Random(4)
    for _ in range(140):
        x, y = rng.randrange(6, 170), rng.randrange(6, 160)
        d.point((x, y), fill=(208, 224, 204, 255))

    def slot(x, y, big=False):
        if big:
            rounded(d, [x - 5, y - 5, x + 20, y + 20], 6, (66, 110, 92, 255), (40, 70, 58, 255))
        rounded(d, [x - 1, y - 1, x + 16, y + 16], 4, (64, 92, 84, 255))
        rounded(d, [x, y, x + 15, y + 15], 3, (238, 244, 232, 255))
        d.line([x + 1, y + 1, x + 14, y + 1], fill=(205, 215, 200, 255))

    for i in range(4):
        slot(8, 8 + i * 18)
    slot(77, 62)
    for y in range(3):
        for x in range(9):
            slot(8 + x * 18, 84 + y * 18)
    rounded(d, [5, 139, 170, 161], 6, (200, 220, 196, 255))
    for x in range(9):
        slot(8 + x * 18, 142)
    for y in range(2):
        for x in range(2):
            slot(98 + x * 18, 18 + y * 18)
    slot(154, 28, big=True)
    d.polygon([(135, 31), (143, 31), (143, 28), (148, 35), (143, 42), (143, 39), (135, 39)], fill=(232, 120, 52, 255))
    # the player's window
    rounded(d, [25, 7, 75, 78], 5, (40, 56, 52, 255), (64, 92, 84, 255))
    for y in range(8, 78):
        d.line([26, y, 74, y], fill=mix((70, 120, 160), (128, 186, 118), (y - 8) / 70))
    return img


def item_whistle(rng):
    img = Image.new("RGBA", (16, 16), CLEAR)
    d = ImageDraw.Draw(img)
    # a round copper whistle with an iron mouthpiece
    d.ellipse([5, 5, 14, 14], fill=(200, 110, 60, 255), outline=(120, 60, 30, 255))
    d.ellipse([8, 8, 11, 11], fill=(90, 46, 24, 255))
    d.rectangle([1, 6, 7, 9], fill=(200, 204, 210, 255), outline=(110, 114, 122, 255))
    d.point((6, 6), fill=(255, 255, 255, 255))
    d.line([10, 4, 13, 1], fill=(220, 40, 40, 255))  # red cord
    d.point((14, 1), fill=(220, 40, 40, 255))
    return img


def spawn_egg(base, spots):
    img = Image.new("RGBA", (16, 16), CLEAR)
    d = ImageDraw.Draw(img)
    d.ellipse([3, 1, 12, 15], fill=base + (255,), outline=shade(base, 0.55))
    rng = random.Random(sum(base))
    for _ in range(7):
        x, y = rng.randrange(5, 11), rng.randrange(3, 14)
        d.rectangle([x, y, x + 1, y + 1], fill=spots + (255,))
    d.point((6, 4), fill=(255, 255, 255, 255))
    return img


def mod_icon():
    img = Image.new("RGBA", (128, 128), CLEAR)
    d = ImageDraw.Draw(img)
    rounded(d, [0, 0, 127, 127], 20, (120, 196, 100, 255))
    d.rectangle([0, 92, 127, 127], fill=(110, 76, 50, 255))
    d.rectangle([0, 88, 127, 96], fill=(90, 160, 70, 255))
    # three Pikmin, Minecraft-blocky
    for i, (name, x) in enumerate((("red", 22), ("yellow", 52), ("blue", 82))):
        c = TYPES[name]
        d.rectangle([x, 50, x + 24, 74], fill=c + (255,), outline=shade(c, 0.6))
        d.rectangle([x + 8, 74, x + 16, 88], fill=shade(c, 0.85))
        d.rectangle([x + 5, 57, x + 9, 63], fill=(250, 250, 250, 255))
        d.rectangle([x + 15, 57, x + 19, 63], fill=(250, 250, 250, 255))
        d.rectangle([x + 7, 60, x + 9, 63], fill=(20, 20, 24, 255))
        d.rectangle([x + 15, 60, x + 17, 63], fill=(20, 20, 24, 255))
        d.line([x + 12, 50, x + 12, 34], fill=(96, 150, 58, 255), width=3)
        d.polygon([(x + 12, 24), (x + 20, 30), (x + 12, 37), (x + 4, 30)], fill=(74, 170, 60, 255))
    return img


# ---------------------------------------------------------------------------------------- sounds

RATE = 44100


def tone(freqs, length, volume=0.5, attack=0.01, release=0.08, wave_kind="sine", vibrato=0.0):
    t = np.linspace(0, length, int(RATE * length), endpoint=False)
    f = np.interp(t, np.linspace(0, length, len(freqs)), freqs)
    if vibrato:
        f = f * (1 + vibrato * np.sin(2 * np.pi * 6 * t))
    phase = 2 * np.pi * np.cumsum(f) / RATE
    if wave_kind == "square":
        s = np.sign(np.sin(phase)) * 0.6
    elif wave_kind == "tri":
        s = 2 / np.pi * np.arcsin(np.sin(phase))
    else:
        s = np.sin(phase) + 0.25 * np.sin(2 * phase)
    env = np.minimum(1, t / max(attack, 1e-4)) * np.minimum(1, (length - t) / max(release, 1e-4))
    return s * env * volume


def noise(length, volume=0.4, low=0.0, decay=8.0, seed=0):
    rng = np.random.default_rng(seed)
    s = rng.uniform(-1, 1, int(RATE * length))
    if low:
        s = np.convolve(s, np.ones(int(low)) / low, mode="same")
    t = np.linspace(0, length, len(s), endpoint=False)
    return s * np.exp(-decay * t) * volume


def concat(*parts):
    return np.concatenate(parts)


def write_ogg(samples, name):
    path = os.path.join(ASSETS, "sounds", *name.split("/")) + ".ogg"
    os.makedirs(os.path.dirname(path), exist_ok=True)
    samples = np.clip(samples, -1, 1)
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as tmp:
        wav_path = tmp.name
    with wave.open(wav_path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes((samples * 32767).astype(np.int16).tobytes())
    subprocess.run(["oggenc", "-Q", "-q", "4", "-o", path, wav_path], check=True)
    os.remove(wav_path)


def voice(base, length, seed, up=True):
    """A tiny high-pitched creature voice."""
    rng = random.Random(seed)
    a, b = base * rng.uniform(0.95, 1.05), base * (1.35 if up else 0.75)
    return tone([a, b, a * 1.1], length, 0.45, 0.01, 0.06, "tri", vibrato=0.03)


def sounds():
    write_ogg(concat(tone([2200, 2350], 0.18, 0.4, 0.01, 0.03, "sine", 0.02), tone([2350, 2350], 0.25, 0.4, 0.005, 0.12, "sine", 0.04)),
              "whistle")
    write_ogg(concat(noise(0.08, 0.5, 6, 30), voice(900, 0.18, 1)), "pikmin/pluck")
    write_ogg(concat(noise(0.06, 0.3, 3, 40, 2), voice(1100, 0.12, 2)), "pikmin/throw")
    write_ogg(voice(980, 0.2, 3), "pikmin/hey")
    write_ogg(concat(noise(0.05, 0.6, 2, 60, 4), tone([500, 380], 0.06, 0.3, 0.002, 0.03, "square")), "pikmin/attack")
    write_ogg(concat(voice(1200, 0.12, 5, up=False), tone([900, 300], 0.35, 0.35, 0.01, 0.2, "tri", 0.06)), "pikmin/die")
    write_ogg(concat(voice(780, 0.14, 6), voice(860, 0.14, 7), voice(820, 0.18, 8, up=False)), "pikmin/carry")
    write_ogg(concat(tone([600, 900], 0.1, 0.3, 0.01, 0.05, "sine"), tone([900, 1300], 0.12, 0.3, 0.01, 0.06, "sine")), "pikmin/drink")
    write_ogg(concat(tone([110, 220, 330], 0.9, 0.5, 0.2, 0.3, "tri", 0.02) + tone([165, 330, 495], 0.9, 0.3, 0.3, 0.3, "sine", 0.02),
                     tone([440, 660], 0.6, 0.35, 0.02, 0.4, "sine", 0.03)), "onion/awaken")
    write_ogg(concat(tone([300, 700], 0.08, 0.4, 0.005, 0.02, "sine"), noise(0.12, 0.25, 4, 25, 9)), "onion/seed")
    write_ogg(concat(tone([900, 200], 0.4, 0.4, 0.02, 0.15, "tri"), noise(0.2, 0.15, 8, 12, 10)), "onion/suck")
    write_ogg(tone([140, 180, 260, 200], 1.6, 0.35, 0.3, 0.6, "tri", 0.05), "onion/fly")
    write_ogg(concat(noise(0.05, 0.5, 2, 50, 11), tone([700, 500], 0.12, 0.3, 0.002, 0.08, "sine")), "posy/hit")
    write_ogg(concat(tone([500, 250], 0.1, 0.4, 0.002, 0.06, "square"), noise(0.1, 0.3, 10, 30, 12)), "pellet/drop")
    write_ogg(concat(noise(0.12, 0.7, 2, 25, 13), tone([1200, 1600], 0.1, 0.2, 0.01, 0.05, "sine")), "egg/break")


SOUND_EVENTS = ["whistle", "pikmin.pluck", "pikmin.throw", "pikmin.hey", "pikmin.attack", "pikmin.die", "pikmin.carry", "pikmin.drink",
                "onion.awaken", "onion.seed", "onion.suck", "onion.fly", "posy.hit", "pellet.drop", "egg.break"]


def sounds_json():
    data = {}
    for event in SOUND_EVENTS:
        path = event.replace(".", "/") if event != "whistle" else "whistle"
        data[event] = {"subtitle": "subtitles.pikcraft." + event, "sounds": ["pikcraft:" + path]}
    with open(os.path.join(ASSETS, "sounds.json"), "w") as f:
        json.dump(data, f, indent=2)
        f.write("\n")


# ---------------------------------------------------------------------------------------- items

def item_model(name):
    with open(os.path.join(ASSETS, "items", name + ".json"), "w") as f:
        json.dump({"model": {"type": "minecraft:model", "model": "pikcraft:item/" + name}}, f, indent=2)
        f.write("\n")
    with open(os.path.join(ASSETS, "models", "item", name + ".json"), "w") as f:
        json.dump({"parent": "minecraft:item/generated", "textures": {"layer0": "pikcraft:item/" + name}}, f, indent=2)
        f.write("\n")


def main():
    rng = random.Random(1)
    os.makedirs(os.path.join(ASSETS, "items"), exist_ok=True)
    os.makedirs(os.path.join(ASSETS, "models", "item"), exist_ok=True)
    icons = Image.new("RGBA", (16 * len(TYPES), 16), CLEAR)
    for i, (name, colour) in enumerate(TYPES.items()):
        save(pikmin_texture(name, colour, rng), TEX, "entity", "pikmin", name + ".png")
        save(onion_texture(name, colour, rng), TEX, "entity", "onion", name + ".png")
        icons.paste(pikmin_icon(colour, name), (i * 16, 0))
        egg = spawn_egg(colour, shade(colour, 0.6)[:3] if name != "white" else (200, 40, 50))
        save(egg, TEX, "item", name + "_pikmin_spawn_egg.png")
        item_model(name + "_pikmin_spawn_egg")
    save(icons, TEX, "gui", "pikmin_icons.png")
    for value in (1, 5, 10, 20):
        save(pellet_texture(value, rng), TEX, "entity", "pellet_%d.png" % value)
    save(seed_texture(rng), TEX, "entity", "seed.png")
    save(posy_texture(rng), TEX, "entity", "pellet_posy.png")
    save(nectar_egg_texture(rng), TEX, "entity", "nectar_egg.png")
    save(nectar_texture(rng), TEX, "entity", "nectar.png")
    save(health_wheel(), TEX, "gui", "health_wheel.png")
    save(day_meter(), TEX, "gui", "day_meter.png")
    save(onion_menu(), TEX, "gui", "onion_menu.png")
    save(inventory_background(), ROOT, "minecraft", "textures", "gui", "container", "inventory.png")
    save(item_whistle(rng), TEX, "item", "whistle.png")
    item_model("whistle")
    for name, base, spots in (("onion", (220, 60, 50), (250, 240, 200)), ("pellet_posy", (90, 160, 60), (250, 210, 60)),
                              ("nectar_egg", (246, 226, 204), (236, 120, 160))):
        save(spawn_egg(base, spots), TEX, "item", name + "_spawn_egg.png")
        item_model(name + "_spawn_egg")
    save(mod_icon(), ASSETS, "icon.png")
    sounds()
    sounds_json()


if __name__ == "__main__":
    main()
