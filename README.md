# PikCraft

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

## How it works

The patcher reads each area's collision surface and turns it into a block heightmap. On the PAL
disc it also patches the game's code (built from the
[projectPiki decompilation](https://github.com/projectPiki/pikmin)), so the game draws the blocks
itself, takes its collision from them and handles breaking and placing. Enemies, ship parts and
carrying routes are unchanged, so every area can still be completed.

See [`pikmin1/README.md`](pikmin1/README.md) for all the options and details.

No Nintendo or Mojang files are in this repository or its releases. The patcher only rewrites the
disc image you give it, the block textures are original pixel art, and the code patch only stores
differences from your own copy of the game. The screenshots show the mod running in Dolphin.

## The original PikCraft (standalone game)

PikCraft started as a standalone game written in Rust, which is still in this repository (`src/`).

A block-world sandbox crossed with squad-commanding strategy. You mine and build in a
procedurally generated voxel world, and you lead a squad of little plant creatures,
**sprouts**, who dig, fight and haul loot for you.

![Your squad follows you](docs/squad.jpg)

### Goal

Five **Star Shards** are hidden around the world. Each one is too heavy for you to carry,
so your sprouts have to haul it back to the **Nest**. Follow the compass at the top of the
screen and the golden light beams. The Nest has a green beam.

Each shard poses a different challenge:

| Shard | Challenge |
|-------|-----------|
| Meadow | A big chomper sleeps next to it |
| Hill top | High up in the mountains |
| Lava moat | Only **red** sprouts survive fire (or bridge it with blocks) |
| Lake island | Only **blue** sprouts can swim (or build a bridge) |
| Buried vault | Deep under a crystal pillar. Dig down to it |

### Sprouts

* **Red**: fire-proof, hit twice as hard.
* **Yellow**: thrown higher, dig twice as fast.
* **Blue**: can swim. Others drown in deep water.

Buried sprouts show a leaf poking out of the grass. Walk up and press **E** to pluck them.
Sprouts grow from leaf to bud to flower over time and get faster as they mature.

Throw a sprout at:

* a **block**: it digs the block out and keeps digging the same kind of block nearby.
  Other sprouts carry the drops home, and they end up in your bag.
* a **chomper**: it latches on and attacks.
* an **item**: it helps carry it. The label shows `carriers/weight`.

Carriers find their own way back to the Nest and dig a tunnel when there's no other route.

![Sprouts digging](docs/digging.jpg)

### Chompers

Chompers sleep during the day and wake when you get close. At night they all wake up and
more of them roam. Those night chompers melt at sunrise. If your sprouts carry a defeated
chomper to the Nest, it grows new sprout seeds in the colour of the sprouts that carried it.

![Fighting a chomper](docs/attack.jpg)

### Controls

| Key | Action |
|-----|--------|
| WASD (QWERTY) or ZQSD (AZERTY) / Space / Shift | Move / jump & swim / sprint |
| Mouse | Look |
| 1 | Squad slot: **left-click** throws a sprout, **right-click** whistles |
| 2-9, mouse wheel | Block slots: **left-click** mines or attacks, **right-click** places |
| F | Throw a sprout (from any slot) |
| R (hold) | Whistle: recall sprouts inside the circle |
| E | Pluck a buried sprout |
| X | Dismiss the squad |
| Tab | Change which colour you throw |
| C | Crafting (planks, pickaxes, glass, bricks, glow lamps, sprout seeds) |
| F1 / F3 | Help / info overlay (position, FPS, settings) |
| L | Switch keyboard layout (QWERTY / AZERTY) |
| M / B | Music on/off / sound effects on/off |
| U | Install an available update (title screen or pause menu) |
| Esc | Pause (Q quits from the pause screen) |

#### Keyboard layouts

PikCraft supports **QWERTY** and **AZERTY** keyboards. On first launch it guesses from your
system (a French or Belgian keyboard or locale selects AZERTY). The title screen shows the
current layout, and you can press **L** there or in game to switch. With AZERTY you move with
**ZQSD**, and the number row (`& é " ' ( - è _ ç`) selects hotbar slots, so you don't need Shift.

Your choices for layout, music and sounds are saved to
`%APPDATA%\PikCraft\settings.txt` (Windows) or `~/.config/PikCraft/settings.txt` (Linux).

### Sound

All sound effects and both music tracks are synthesized in code when the game starts, so
nothing is loaded from audio files. You'll hear a cheerful day tune and a slower night theme
that crossfade with the sun. Plucking, throwing, digging, the whistle, chompers, deliveries
and Star Shards each have their own sound, and sounds get quieter the further away they are.

![Night with a glow lamp](docs/night.jpg)

### Updates

This game has a built-in updater that checks this repository's releases. Those now hold the
Pikmin 1 patcher, so it no longer finds a newer version of this game.

### Download & run

There is no prebuilt release of this game any more: build it from source (below). It runs on
Windows 10/11 (x64) and Linux (x86_64, glibc 2.34+; needs an OpenGL driver plus `libGL`, `libX11`,
`libXi` and `libasound`).

Command-line options: `--seed N` plays a specific world, and `--version` prints the version.

### Building from source

Install [Rust](https://rustup.rs), then:

```sh
cargo run --release
```

On Linux, building needs the ALSA headers (`sudo apt install libasound2-dev`). X11 and OpenGL
are loaded at runtime. On Windows nothing extra is needed.

To cross-compile a Windows build from Linux:

```sh
rustup target add x86_64-pc-windows-gnu
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
```

#### Tests

```sh
cargo test --release                                    # world gen, lighting, path finding, sprout AI, winnability, audio, layouts
xvfb-run -a ./target/release/pikcraft --selftest out    # scripted play-through, saves screenshots to out/
./target/release/pikcraft --export-audio sounds         # writes every synthesized sound and both music loops as WAV
```

`--selftest` plucks a squad, throws it at blocks, whistles it back, fights a chomper and
checks each step, including that the matching sounds fire. It exits with a non-zero code if
something fails. CI runs it on both Linux and Windows.

### How it's made

PikCraft is written in Rust on top of [macroquad](https://macroquad.rs). Every texture,
model and the window icon is generated in code, so the game ships as one small executable
with no asset files.

* `src/world.rs`: 160×160×64 voxel world, terrain/cave/ore generation, landmark placement,
  incremental sky and block light propagation, voxel ray casts, A* path finding (with digging).
* `src/render.rs`: procedural texture atlas, chunk meshing with ambient occlusion and smooth
  lighting, and a custom GPU pipeline with fog and day/night.
* `src/game.rs`: player, sprout AI, chompers, carrying, nest, crafting, day/night cycle.
* `src/draw.rs`: creature models, effects and HUD.
* `src/audio.rs`: a small synthesizer (oscillators, envelopes, filtered noise), the sound
  effects, the procedurally composed day/night music and playback with crossfading.
* `src/controls.rs`: QWERTY/AZERTY handling and saved settings.
* `src/main.rs`: window, input, title screen, game loop and the self-test.

PikCraft is a fan-made homage to block-building sandboxes and plant-squad strategy games.
It is not affiliated with or endorsed by Mojang, Microsoft or Nintendo, and it uses no
assets from their games.
