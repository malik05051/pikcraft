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

from . import dolpatch, gcdisc, i18n, mapbuild
from . import modfile as mf
from .i18n import tr

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


def _error(message: str) -> int:
    print(tr("error", message=message), file=sys.stderr)
    return 1


def main(argv: list[str] | None = None) -> int:
    # The language has to be known before the help text is built.
    pre = argparse.ArgumentParser(add_help=False)
    pre.add_argument("--lang")
    lang = pre.parse_known_args(argv)[0].lang
    i18n.set_language(lang if lang in i18n.LANGUAGES else i18n.detect())
    argparse._ = i18n.gettext  # argparse's own words ("usage:", "options", ...)

    ap = argparse.ArgumentParser(prog="pikcraft-patcher", description=tr("description"), add_help=False)
    ap.add_argument("source", nargs="?", help=tr("help_source"))
    ap.add_argument("output", nargs="?", help=tr("help_output"))
    ap.add_argument("-h", "--help", action="help", help=tr("help_help"))
    ap.add_argument("--block-size", type=float, default=DEFAULT_BLOCK, help=tr("help_block_size"))
    ap.add_argument("--keep-original", action="store_true", help=tr("help_keep_original"))
    ap.add_argument("--terrain-only", action="store_true", help=tr("help_terrain_only"))
    ap.add_argument("--only", action="append", default=[], help=tr("help_only"))
    ap.add_argument("--list", action="store_true", help=tr("help_list"))
    ap.add_argument("--lang", choices=i18n.LANGUAGES, help=tr("help_lang"))
    ap.add_argument(
        "--version",
        action="version",
        version=tr("version", version=__version__, versions=", ".join(code_versions()) or tr("none")),
        help=tr("help_version"),
    )
    args = ap.parse_args(argv)

    if not args.source:
        args.source = input(tr("prompt_source")).strip().strip('"')
    try:
        disc = gcdisc.GameCubeDisc(args.source)
    except FileNotFoundError:
        return _error(tr("file_not_found", path=args.source))
    except (OSError, gcdisc.DiscError) as e:
        return _error(str(e))
    with disc:
        print(f"{disc.title} [{disc.version_tag}]")
        if disc.game_id not in SUPPORTED:
            return _error(tr("not_pikmin", game_id=disc.game_id, expected=", ".join(sorted(SUPPORTED))))
        maps = [m for m in find_maps(disc) if not args.only or any(o.lower() in m.lower() for o in args.only)]
        if args.list or not maps:
            for m in maps:
                print(" ", m)
            if not maps:
                return _error(tr("no_maps"))
            return 0
        if not args.output:
            args.output = input(tr("prompt_output")).strip().strip('"')
        code = None if args.terrain_only or args.keep_original else code_patch(disc)
        new_dol = None
        if code:
            try:
                new_dol = dolpatch.apply(*code)
            except dolpatch.PatchError as e:
                return _error(str(e))
            print(tr("mode_full"))
        else:
            print(tr("mode_terrain"))
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
                print(tr("skipped", path=path, reason=e))
                continue
            replacements[path] = out
            faces = tr("faces", count=rep.quads) if not new_dol else ""
            print(
                tr(
                    "map_done",
                    path=path,
                    columns=rep.columns,
                    faces=faces,
                    block=i18n.number(rep.block_size, 0),
                    before=len(data) // 1024,
                    after=len(out) // 1024,
                    seconds=i18n.number(time.time() - t),
                )
            )
    if not replacements:
        return _error(tr("nothing_converted"))
    try:
        gcdisc.write_patched(args.source, args.output, replacements, dol=new_dol)
    except (OSError, gcdisc.DiscError) as e:
        return _error(str(e))
    print(tr("done", output=args.output))
    return 0


if __name__ == "__main__":
    sys.exit(main())
