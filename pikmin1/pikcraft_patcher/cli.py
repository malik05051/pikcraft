"""PikCraft patcher: turns your own Pikmin (GameCube) disc image into PikCraft.

    pikcraft-patcher "Pikmin.iso" "PikCraft.iso"

Nothing from the game is shipped with this tool; it only rewrites the map models on the disc you
give it, and writes the result to a new file.
"""

from __future__ import annotations

import argparse
import re
import struct
import sys
import time

from . import gcdisc, mapbuild
from . import modfile as mf

__version__ = "1.0.0"

SUPPORTED = {"GPIE01", "GPIJ01", "GPIP01"}
DEFAULT_BLOCK = 20.0  # map units per block (a Pikmin is roughly 10 units tall)
DATA_ROOT = "dataDir/"


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
    ap.add_argument("--keep-original", action="store_true", help="draw the original terrain under the blocks")
    ap.add_argument("--only", action="append", default=[], help="only convert maps whose path contains this text")
    ap.add_argument("--list", action="store_true", help="list the map models found and exit")
    ap.add_argument("--version", action="version", version=f"%(prog)s {__version__}")
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
        replacements = {}
        for path in maps:
            t = time.time()
            data = disc.read(path)
            try:
                out, rep = mapbuild.build(data, args.block_size, args.keep_original)
            except (mapbuild.BuildError, mf.ModError, struct.error, IndexError, ValueError) as e:
                print(f"  {path}: skipped ({e})")
                continue
            replacements[path] = out
            print(
                f"  {path}: {rep.columns} columns, {rep.quads} faces, block {rep.block_size:g}, "
                f"{len(data) // 1024} -> {len(out) // 1024} KiB ({time.time() - t:.1f}s)"
            )
    if not replacements:
        print("error: no maps could be converted", file=sys.stderr)
        return 1
    try:
        gcdisc.write_patched(args.source, args.output, replacements)
    except (OSError, gcdisc.DiscError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    print(f"Done: {args.output}  (open it in Dolphin)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
