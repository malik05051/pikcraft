# PikCraft

**English** | [Français](README.fr.md)

## AI Disclosure : This whole game has been vibecoded by Claude Code Opus 5.5.

**Minecraft inside Nintendo's real Pikmin 1.** PikCraft patches your own Pikmin (GameCube) disc
so every area is rebuilt from grass, dirt, stone, sand, planks, mud and water blocks, and you can
break and place blocks while Olimar and the Pikmin walk in and out of the holes you dig. You play
it in [Dolphin](https://dolphin-emu.org).

![The Impact Site rebuilt from blocks](docs/pikmin1-impact-site.jpg)

## Play it

1. Download the patcher for your system from the
   [Releases page](https://github.com/malik05051/pikcraft/releases): `pikcraft-patcher-windows-x86_64.exe`
   or `pikcraft-patcher-linux-x86_64`.
2. Run it with your Pikmin disc image and a name for the new one:

   ```
   pikcraft-patcher "Pikmin.iso" "PikCraft.iso"
   ```

   Run it with no arguments and it asks for both. Your original image is never modified.
3. Open `PikCraft.iso` in Dolphin.

You need your own copy of the game as a plain `.iso`/`.gcm`. If yours is `.rvz` or `.gcz`,
right-click it in Dolphin, choose **Convert File…** and pick **ISO**. Breaking and placing need
the PAL disc (GPIP01); NTSC-U (GPIE01) and NTSC-J (GPIJ01) discs get the block terrain without
them.

## Break and place blocks

| Button | Action |
| --- | --- |
| D-pad ◀ | Break the top block under the cursor |
| D-pad ▶ | Place a block there (the last type you broke; dirt to start with) |

Everything else is Pikmin as usual. A white outline shows which block the cursor is on.

![Aim with the cursor, then break the block](docs/pikmin1-break.jpg)

Holes and new blocks change the ground straight away: Olimar and the Pikmin walk into the holes
you dig and onto what you build. A one-block step is a slope you can walk up; anything higher is a
wall. The terrain goes back to normal each day.

![A three-block pillar next to a hole](docs/pikmin1-build.jpg)

## Playing in French

- **The game:** the PAL disc (GPIP01) includes French. The game uses the console's language: in
  Dolphin, go to **Config > GameCube > System Language** and pick **French**. If you already have a
  save, change the language in **Options** on the game's title screen. NTSC-U and NTSC-J discs are
  English or Japanese only.
- **The patcher** speaks French when your system is set to French. Use `--lang fr` or `--lang en`
  to choose.

## How it works

The patcher reads each area's collision surface and turns it into a block heightmap. On the PAL
disc it also patches the game's code (built from the
[projectPiki decompilation](https://github.com/projectPiki/pikmin)), so the game draws the blocks
itself, takes its collision from them and handles breaking and placing. Enemies, ship parts and
carrying routes are unchanged, so every area can still be completed.

See [`pikmin1/README.md`](pikmin1/README.md) for all the options and details
([en français](pikmin1/README.fr.md)).

No Nintendo or Mojang files are in this repository or its releases. The patcher only rewrites the
disc image you give it, the block textures are original pixel art, and the code patch only stores
differences from your own copy of the game. The screenshots show the mod running in Dolphin.
