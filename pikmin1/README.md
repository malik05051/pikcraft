# PikCraft for Pikmin 1 (GameCube)

PikCraft running inside Nintendo's real Pikmin 1 engine. The patcher rebuilds each level's map
model as Minecraft-style blocks and writes a new disc image you play in [Dolphin](https://dolphin-emu.org).

> Tested on the PAL disc (GPIP01) in Dolphin: the Impact Site plays, and blocks break and place.
> The other areas convert the same way but haven't been played through yet.

## Controls

| Button | Action |
| --- | --- |
| D-pad ◀ | Break the top block under the cursor |
| D-pad ▶ | Place a block there (the last type you broke; dirt to start with) |

Everything else is Pikmin as usual. Olimar and the Pikmin walk into the holes you dig and onto the
blocks you place. The terrain resets each day. Breaking and placing needs the PAL disc (GPIP01,
revision 0); with other versions the patcher builds the block terrain without them.

## What it does

- It probes every level's collision surface on a block grid, the same surface Pikmin and Olimar
  walk on. Ground becomes grass, dirt and stone; rock becomes stone; wood becomes planks; mud stays
  mud. Pond beds (marked as water in the collision) become sand, filled with water up to the shore.
- **Full mode** (PAL): the game executable is patched with the PikCraft code (`mod/`). It draws the
  blocks itself every frame, answers the map's collision from the block heightmap (slopes between
  block centres, so a one-block step is walkable and higher cliffs are walls), and handles breaking
  and placing. The heightmap travels inside each map model as an extra chunk.
- **Terrain-only mode** (other versions): the visible faces are greedy-merged into the map model
  itself; the original collision is kept.
- Textures are original 16×16 pixel art. No Mojang assets are used.
- Enemies, ship parts, carrying routes and lighting are unchanged, so every level is still
  completable.

## Use it

You need your own Pikmin disc image: NTSC-U (GPIE01), NTSC-J (GPIJ01) or PAL (GPIP01), as a plain
`.iso`/`.gcm`. If yours is `.rvz` or `.gcz`, convert it in Dolphin: right-click the game,
choose **Convert File…** and pick **ISO**.

```
pikcraft-patcher "Pikmin.iso" "PikCraft.iso"
```

Run it without arguments to be asked for the paths. Your original image is never modified.

| Option | Effect |
| --- | --- |
| `--block-size 20` | Block size in map units; smaller is more detailed. A map that would need too many vertices automatically uses bigger blocks. |
| `--only <text>` | Convert only maps whose path contains the text. |
| `--list` | List the map models found and exit. |
| `--terrain-only` | Don't patch the game code (no breaking or placing). |
| `--keep-original` | Terrain-only mode, also drawing the original terrain for comparison. |

From source (Python 3.10+, no dependencies): `python -m pikcraft_patcher …`; tests:
`python -m unittest discover -s tests -t .`

## Legal

This repository and its releases contain no Nintendo code, data or assets. The patcher only reads
the disc image you supply; please dump your own copy of the game. The code patch
(`pikcraft_patcher/data/*.dolpatch`) stores only differences from your own executable, so it is
useless without it; `mod/build_dolpatch.sh` rebuilds it from the
[projectPiki decompilation](https://github.com/projectPiki/pikmin) and the sources in `mod/`.

## How it works

- `pikcraft_patcher/gcdisc.py` reads the disc's file table and writes a patched copy. It appends
  replaced files and repoints their entries.
- `pikcraft_patcher/modfile.py` reads and writes the `.mod` model format, following the
  [projectPiki decompilation](https://github.com/projectPiki/pikmin) (`BaseShape::read`).
- `pikcraft_patcher/voxel.py` turns collision triangles into a block grid and a greedy face mesh.
- `pikcraft_patcher/mapbuild.py` grafts the block meshes, textures and materials into the map
  model. Each block material is a copy of one of the map's own materials (opaque, or translucent
  for water), so the renderer sees nothing new. It also matches the map's triangle winding.
- `pikcraft_patcher/dolpatch.py` makes and applies the executable patch (bsdiff-style word
  differences, checked by SHA-1 on both ends).
- `mod/PikCraft.h` and `mod/pikcraft.inl` are the game code: block rendering with GX, heightmap
  collision behind `BaseShape::getCollTris`, breaking and placing. `mod/decomp-hooks.patch` holds
  the few lines that hook them into the game.
- `pikcraft_patcher/textures.py` holds the procedural block textures and the GameCube RGB565/RGB5A3
  encoders.
