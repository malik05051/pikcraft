# PikCraft for Pikmin 1 (GameCube)

PikCraft running inside Nintendo's real Pikmin 1 engine. The patcher rebuilds each level's map
model as Minecraft-style blocks and writes a new disc image you play in [Dolphin](https://dolphin-emu.org).

> **Early version.** The block terrain has been tested on the PAL disc (GPIP01) in Dolphin: the
> Impact Site plays fine. The other areas convert the same way but haven't been played through
> yet. Breaking and placing blocks in game needs code changes and will come later.

## What it does

- It probes every level's collision surface on a block grid, the same surface Pikmin and Olimar
  walk on. Ground becomes grass, dirt and stone; rock becomes stone; wood becomes planks; mud stays
  mud. Pond beds (marked as water in the collision) become sand, filled with water up to the shore.
- It greedy-merges the visible faces and textures them with original 16×16 pixel art. No Mojang
  assets are used.
- The original terrain is no longer drawn. Collision, enemies, ship parts, carrying routes and
  lighting are unchanged, so every level is still completable.

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
| `--keep-original` | Also draw the original terrain, for comparison. |

From source (Python 3.10+, no dependencies): `python -m pikcraft_patcher …`; tests:
`python -m unittest discover -s tests -t .`

## Legal

This repository and its releases contain no Nintendo code, data or assets. The patcher only reads
the disc image you supply; please dump your own copy of the game.

## How it works

- `pikcraft_patcher/gcdisc.py` reads the disc's file table and writes a patched copy. It appends
  replaced files and repoints their entries.
- `pikcraft_patcher/modfile.py` reads and writes the `.mod` model format, following the
  [projectPiki decompilation](https://github.com/projectPiki/pikmin) (`BaseShape::read`).
- `pikcraft_patcher/voxel.py` turns collision triangles into a block grid and a greedy face mesh.
- `pikcraft_patcher/mapbuild.py` grafts the block meshes, textures and materials into the map
  model. Each block material is a copy of one of the map's own materials (opaque, or translucent
  for water), so the renderer sees nothing new. It also matches the map's triangle winding.
- `pikcraft_patcher/textures.py` holds the procedural block textures and the GameCube RGB565/RGB5A3
  encoders.
