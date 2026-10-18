"""PikCraft patcher: turns your own Pikmin (GameCube) disc image into PikCraft.

    pikcraft-patcher "Pikmin.iso" "PikCraft.iso"

Nothing from the game is shipped with this tool; it only rewrites the map models (and, for versions
with a code patch, the game executable) on the disc you give it, and writes the result to a new file.
"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import struct
import sys
import time

from . import dolpatch, gcdisc, mapbuild
from . import modfile as mf

__version__ = "1.0.0"

SUPPORTED = {"GPIE01", "GPIJ01", "GPIP01"}
DEFAULT_BLOCK = 20.0  # map units per block (a Pikmin is roughly 10 units tall)
DATA_ROOT = "dataDir/"
PATCH_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "data")


def code_versions() -> list[str]:
    """Disc versions this patcher has block-breaking code for."""
    if not os.path.isdir(PATCH_DIR):
        return []
    return sorted(f[: -len(".dolpatch")] for f in os.listdir(PATCH_DIR) if f.endswith(".dolpatch"))


def code_patch(disc: gcdisc.GameCubeDisc) -> tuple[bytes, bytes] | None:
    """(original executable, executable patch) when this disc's version has the block-breaking code."""
    path = os.path.join(PATCH_DIR, f"{disc.version_tag}.dolpatch")
    if not os.path.exists(path):
        return None
    with open(path, "rb") as f:
        patch = f.read()
    dol = disc.read_dol()
    if hashlib.sha1(dol).hexdigest() != dolpatch.source_sha1(patch):
        return None
    return dol, patch


def find_maps(disc: gcdisc.GameCubeDisc) -> list[str]:
    """Disc paths of every map model named by a stage file (`map_file <path>`)."""
    maps: list[str] = []
    for key, f in sorted(disc.files.items()):
        if "/stages/" not in "/" + key or not key.endswith(".ini"):
            continue
        text = disc.read(f.path).decode("latin-1", "replace")
        for m in re.finditer(r"\bmap_file\s+\"?([^\s\"]+)", text):
            rel = m.group(1)
            for candidate in (DATA_ROOT + rel, rel):
                if candidate.lower() in disc.files:
                    path = disc.files[candidate.lower()].path
                    if path not in maps:
                        maps.append(path)
                    break
    if not maps:
        # Fall back to any model carrying a collision prism (only maps have one).
        for key, f in sorted(disc.files.items()):
            if key.endswith(".mod"):
                try:
                    if mf.ModFile.parse(disc.read(f.path)).get(mf.CHUNK_COLLPRISM):
                        maps.append(f.path)
                except (mf.ModError, struct.error, IndexError, ValueError):
                    pass
    return maps


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(prog="pikcraft-patcher", description=__doc__.split("\n\n")[0])
    ap.add_argument("source", nargs="?", help="your Pikmin disc image (.iso / .gcm)")
    ap.add_argument("output", nargs="?", help="where to write PikCraft (a new .iso)")
    ap.add_argument("--block-size", type=float, default=DEFAULT_BLOCK, help="block size in map units (default %(default)s)")
    ap.add_argument("--keep-original", action="store_true", help="draw the original terrain under the blocks (terrain-only mode)")
    ap.add_argument("--terrain-only", action="store_true", help="don't patch the game code: blocks can't be broken or placed")
    ap.add_argument("--only", action="append", default=[], help="only convert maps whose path contains this text")
    ap.add_argument("--list", action="store_true", help="list the map models found and exit")
    ap.add_argument("--version", action="version", version=f"%(prog)s {__version__} (block breaking: {', '.join(code_versions()) or 'none'})")
    args = ap.parse_args(argv)

    if not args.source:
        args.source = input("Path to your Pikmin disc image: ").strip().strip('"')
    try:
        disc = gcdisc.GameCubeDisc(args.source)
    except (OSError, gcdisc.DiscError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    with disc:
        print(f"{disc.title} [{disc.version_tag}]")
        if disc.game_id not in SUPPORTED:
            print(f"error: {disc.game_id} is not Pikmin (expected one of {', '.join(sorted(SUPPORTED))})", file=sys.stderr)
            return 1
        maps = [m for m in find_maps(disc) if not args.only or any(o.lower() in m.lower() for o in args.only)]
        if args.list or not maps:
            for m in maps:
                print(" ", m)
            if not maps:
                print("error: no map models found on this disc", file=sys.stderr)
                return 1
            return 0
        if not args.output:
            args.output = input("Where to save PikCraft (.iso): ").strip().strip('"')
        code = None if args.terrain_only or args.keep_original else code_patch(disc)
        new_dol = None
        if code:
            try:
                new_dol = dolpatch.apply(*code)
            except dolpatch.PatchError as e:
                print(f"error: {e}", file=sys.stderr)
                return 1
            print("Mode: full (break blocks with D-pad left, place with D-pad right)")
        else:
            print("Mode: terrain only (block breaking needs the PAL disc, GPIP01 revision 0)")
        replacements = {}
        for path in maps:
            t = time.time()
            data = disc.read(path)
            try:
                if new_dol:
                    out, rep = mapbuild.build_blocks(data, args.block_size)
                else:
                    out, rep = mapbuild.build(data, args.block_size, args.keep_original)
            except (mapbuild.BuildError, mf.ModError, struct.error, IndexError, ValueError) as e:
                print(f"  {path}: skipped ({e})")
                continue
            replacements[path] = out
            faces = f"{rep.quads} faces, " if not new_dol else ""
            print(
                f"  {path}: {rep.columns} columns, {faces}block {rep.block_size:g}, "
                f"{len(data) // 1024} -> {len(out) // 1024} KiB ({time.time() - t:.1f}s)"
            )
    if not replacements:
        print("error: no maps could be converted", file=sys.stderr)
        return 1
    try:
        gcdisc.write_patched(args.source, args.output, replacements, dol=new_dol)
    except (OSError, gcdisc.DiscError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    print(f"Done: {args.output}  (open it in Dolphin)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
