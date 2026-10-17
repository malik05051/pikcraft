//! Voxel world: storage, terrain generation, incremental lighting, ray casts and path finding.

use crate::block::Block;
use crate::noise::{Rng, fbm2, value3};
use macroquad::math::{IVec3, Vec3, ivec3, vec3};
use std::collections::{BinaryHeap, HashMap, VecDeque};

pub const SX: i32 = 160;
pub const SZ: i32 = 160;
pub const SY: i32 = 64;
pub const CHUNK: i32 = 16;
pub const CX: i32 = SX / CHUNK;
pub const CZ: i32 = SZ / CHUNK;
pub const SEA: i32 = 24;

const DIRS: [IVec3; 6] = [
    IVec3::new(1, 0, 0),
    IVec3::new(-1, 0, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(0, 0, -1),
];

pub struct RayHit {
    pub pos: IVec3,
    pub normal: IVec3,
    pub dist: f32,
}

pub struct World {
    blocks: Vec<u8>,
    sky: Vec<u8>,
    glow: Vec<u8>,
    pub dirty: Vec<bool>,
    pub seed: u32,
    /// Spot reserved for the sprout nest (feet position on the surface).
    pub nest: IVec3,
    /// Where the guarded star shards are generated.
    pub shard_spots: Vec<IVec3>,
    /// Where wild buried sprouts and sleeping chompers start.
    pub sprout_spots: Vec<(IVec3, u8)>,
    pub chomper_spots: Vec<IVec3>,
}

#[inline]
fn idx(x: i32, y: i32, z: i32) -> usize {
    ((z * SX + x) * SY + y) as usize
}

#[inline]
pub fn in_bounds(x: i32, y: i32, z: i32) -> bool {
    x >= 0 && x < SX && y >= 0 && y < SY && z >= 0 && z < SZ
}

/// Extra light lost when travelling into a block.
fn attenuation(b: Block) -> u8 {
    match b {
        Block::Leaves | Block::Water => 2,
        _ => 1,
    }
}

impl World {
    pub fn get(&self, x: i32, y: i32, z: i32) -> Block {
        if y < 0 {
            return Block::Bedrock;
        }
        if !in_bounds(x, y, z) {
            return Block::Air;
        }
        Block::from_u8(self.blocks[idx(x, y, z)])
    }

    pub fn get_v(&self, p: IVec3) -> Block {
        self.get(p.x, p.y, p.z)
    }

    /// Solid for collisions (the world edge is an invisible wall).
    pub fn solid(&self, x: i32, y: i32, z: i32) -> bool {
        if x < 0 || z < 0 || x >= SX || z >= SZ || y < 0 {
            return true;
        }
        self.get(x, y, z).is_solid()
    }

    pub fn sky_light(&self, x: i32, y: i32, z: i32) -> u8 {
        if y >= SY {
            return 15;
        }
        if !in_bounds(x, y, z) {
            return if y < 0 { 0 } else { 15 };
        }
        self.sky[idx(x, y, z)]
    }

    pub fn block_light(&self, x: i32, y: i32, z: i32) -> u8 {
        if !in_bounds(x, y, z) {
            return 0;
        }
        self.glow[idx(x, y, z)]
    }

    /// Light at a point as (sky, block) in 0..1.
    pub fn light_at(&self, p: Vec3) -> (f32, f32) {
        let (x, y, z) = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        (
            self.sky_light(x, y, z) as f32 / 15.0,
            self.block_light(x, y, z) as f32 / 15.0,
        )
    }

    /// Highest solid block in a column (or -1).
    pub fn surface_y(&self, x: i32, z: i32) -> i32 {
        for y in (0..SY).rev() {
            if self.get(x, y, z).is_solid() {
                return y;
            }
        }
        -1
    }

    fn mark_dirty(&mut self, x: i32, z: i32) {
        for dx in [-1, 0, 1] {
            for dz in [-1, 0, 1] {
                let (nx, nz) = (x + dx, z + dz);
                if nx < 0 || nz < 0 || nx >= SX || nz >= SZ {
                    continue;
                }
                let (cx, cz) = (nx / CHUNK, nz / CHUNK);
                self.dirty[(cz * CX + cx) as usize] = true;
            }
        }
    }

    /// Place or remove a block, keeping lighting and meshes up to date.
    pub fn set(&mut self, p: IVec3, b: Block) {
        if !in_bounds(p.x, p.y, p.z) {
            return;
        }
        let i = idx(p.x, p.y, p.z);
        if self.blocks[i] == b as u8 {
            return;
        }
        self.blocks[i] = b as u8;
        self.mark_dirty(p.x, p.z);
        self.relight(p);
    }

    fn relight(&mut self, p: IVec3) {
        // Remove the light that used to pass through this cell, then let neighbours refill it.
        for sky in [true, false] {
            let i = idx(p.x, p.y, p.z);
            let old = if sky { self.sky[i] } else { self.glow[i] };
            let mut add = VecDeque::new();
            if old > 0 {
                let mut rem = VecDeque::new();
                if sky {
                    self.sky[i] = 0;
                } else {
                    self.glow[i] = 0;
                }
                rem.push_back((p, old));
                self.remove_light(sky, &mut rem, &mut add);
            }
            for d in DIRS {
                let n = p + d;
                if in_bounds(n.x, n.y, n.z) {
                    add.push_back(n);
                }
            }
            if sky && p.y == SY - 1 && !self.get_v(p).is_opaque() {
                self.sky[i] = 15;
                add.push_back(p);
            }
            if !sky {
                let e = self.get_v(p).emission();
                if e > 0 {
                    self.glow[i] = e;
                    add.push_back(p);
                }
            }
            self.spread_light(sky, &mut add);
        }
    }

    fn remove_light(
        &mut self,
        sky: bool,
        rem: &mut VecDeque<(IVec3, u8)>,
        add: &mut VecDeque<IVec3>,
    ) {
        while let Some((p, old)) = rem.pop_front() {
            for d in DIRS {
                let n = p + d;
                if !in_bounds(n.x, n.y, n.z) {
                    continue;
                }
                let ni = idx(n.x, n.y, n.z);
                let nl = if sky { self.sky[ni] } else { self.glow[ni] };
                if nl == 0 {
                    continue;
                }
                let sun_column = sky && d.y == -1 && old == 15 && nl == 15;
                if nl < old || sun_column {
                    let e = if sky {
                        0
                    } else {
                        self.get(n.x, n.y, n.z).emission()
                    };
                    if sky {
                        self.sky[ni] = 0;
                    } else {
                        self.glow[ni] = e;
                    }
                    if e > 0 {
                        add.push_back(n);
                    }
                    self.mark_dirty(n.x, n.z);
                    rem.push_back((n, nl));
                } else {
                    add.push_back(n);
                }
            }
        }
    }

    fn spread_light(&mut self, sky: bool, queue: &mut VecDeque<IVec3>) {
        while let Some(p) = queue.pop_front() {
            let pi = idx(p.x, p.y, p.z);
            let level = if sky { self.sky[pi] } else { self.glow[pi] };
            if level <= 1 {
                continue;
            }
            for d in DIRS {
                let n = p + d;
                if !in_bounds(n.x, n.y, n.z) {
                    continue;
                }
                let nb = self.get(n.x, n.y, n.z);
                if nb.is_opaque() {
                    continue;
                }
                let att = attenuation(nb);
                let new = if sky && d.y == -1 && level == 15 && att == 1 {
                    15
                } else {
                    level.saturating_sub(att)
                };
                let ni = idx(n.x, n.y, n.z);
                let cur = if sky { self.sky[ni] } else { self.glow[ni] };
                if new > cur {
                    if sky {
                        self.sky[ni] = new;
                    } else {
                        self.glow[ni] = new;
                    }
                    self.mark_dirty(n.x, n.z);
                    queue.push_back(n);
                }
            }
        }
    }

    fn initial_light(&mut self) {
        let mut q = VecDeque::new();
        for z in 0..SZ {
            for x in 0..SX {
                for y in (0..SY).rev() {
                    let b = self.get(x, y, z);
                    if b.is_opaque() || attenuation(b) > 1 {
                        break;
                    }
                    self.sky[idx(x, y, z)] = 15;
                    q.push_back(ivec3(x, y, z));
                }
            }
        }
        self.spread_light(true, &mut q);
        for z in 0..SZ {
            for x in 0..SX {
                for y in 0..SY {
                    let e = self.get(x, y, z).emission();
                    if e > 0 {
                        self.glow[idx(x, y, z)] = e;
                        q.push_back(ivec3(x, y, z));
                    }
                }
            }
        }
        self.spread_light(false, &mut q);
    }

    // ------------------------------------------------------------------ generation

    pub fn generate(seed: u32) -> World {
        let n = (SX * SY * SZ) as usize;
        let mut w = World {
            blocks: vec![0; n],
            sky: vec![0; n],
            glow: vec![0; n],
            dirty: vec![true; (CX * CZ) as usize],
            seed,
            nest: IVec3::ZERO,
            shard_spots: Vec::new(),
            sprout_spots: Vec::new(),
            chomper_spots: Vec::new(),
        };
        let mut rng = Rng::new(seed as u64);

        // Height map.
        let mut height = vec![0i32; (SX * SZ) as usize];
        let (cxw, czw) = (SX as f32 / 2.0, SZ as f32 / 2.0);
        for z in 0..SZ {
            for x in 0..SX {
                let (fx, fz) = (x as f32, z as f32);
                let base = fbm2(seed, fx / 48.0, fz / 48.0, 5);
                let hills = fbm2(seed ^ 0xA5A5, fx / 90.0, fz / 90.0, 3);
                let mut h = 14.0 + base * 26.0;
                if hills > 0.55 {
                    h += (hills - 0.55) * 75.0;
                }
                // Keep the very centre (the nest meadow) gentle and above water.
                let d = ((fx - cxw).powi(2) + (fz - czw).powi(2)).sqrt();
                if d < 22.0 {
                    let t = (1.0 - d / 22.0).clamp(0.0, 1.0);
                    h = h * (1.0 - t) + (SEA as f32 + 4.0) * t;
                }
                height[(z * SX + x) as usize] = (h as i32).clamp(4, SY - 8);
            }
        }

        for z in 0..SZ {
            for x in 0..SX {
                let h = height[(z * SX + x) as usize];
                for y in 0..SY {
                    let b = if y == 0 {
                        Block::Bedrock
                    } else if y < h - 4 {
                        Block::Stone
                    } else if y < h {
                        if h <= SEA + 1 {
                            Block::Sand
                        } else {
                            Block::Dirt
                        }
                    } else if y == h {
                        if h <= SEA + 1 {
                            if h < SEA - 2 {
                                Block::Gravel
                            } else {
                                Block::Sand
                            }
                        } else if h > 44 {
                            Block::Snow
                        } else {
                            Block::Grass
                        }
                    } else if y <= SEA {
                        Block::Water
                    } else {
                        Block::Air
                    };
                    w.blocks[idx(x, y, z)] = b as u8;
                }
            }
        }

        // Caves: intersecting noise bands make winding tunnels.
        for z in 0..SZ {
            for x in 0..SX {
                let h = height[(z * SX + x) as usize];
                for y in 2..(h - 1).min(SY - 1) {
                    let (fx, fy, fz) = (x as f32, y as f32, z as f32);
                    let a = value3(seed ^ 0x1111, fx / 18.0, fy / 12.0, fz / 18.0);
                    let b = value3(seed ^ 0x2222, fx / 18.0, fy / 12.0, fz / 18.0);
                    let tunnel = (a - 0.5).abs() < 0.045 && (b - 0.5).abs() < 0.08;
                    let cavern =
                        value3(seed ^ 0x3333, fx / 26.0, fy / 14.0, fz / 26.0) > 0.78 && y < 20;
                    // Don't breach the sea floor.
                    if (tunnel || cavern) && !(h <= SEA + 1 && y > h - 4) {
                        let blk = if y <= 5 { Block::Lava } else { Block::Air };
                        w.blocks[idx(x, y, z)] = blk as u8;
                    }
                }
            }
        }

        // Ore veins.
        let veins: [(Block, i32, i32, usize, i32); 3] = [
            (Block::CoalOre, 8, 52, 260, 6),
            (Block::IronOre, 4, 34, 160, 5),
            (Block::CrystalOre, 2, 18, 70, 4),
        ];
        for (ore, lo, hi, count, size) in veins {
            for _ in 0..count {
                let mut p = ivec3(rng.irange(0, SX), rng.irange(lo, hi), rng.irange(0, SZ));
                for _ in 0..rng.irange(2, size + 1) {
                    if in_bounds(p.x, p.y, p.z) && w.get_v(p) == Block::Stone {
                        w.blocks[idx(p.x, p.y, p.z)] = ore as u8;
                    }
                    p += DIRS[rng.irange(0, 6) as usize];
                }
            }
        }

        // Nest meadow at the centre.
        let (nx, nz) = (SX / 2, SZ / 2);
        let ny = height[(nz * SX + nx) as usize];
        for z in nz - 5..=nz + 5 {
            for x in nx - 5..=nx + 5 {
                for y in ny + 1..SY {
                    w.blocks[idx(x, y, z)] = Block::Air as u8;
                }
                for y in ny - 3..ny {
                    w.blocks[idx(x, y, z)] = Block::Dirt as u8;
                }
                w.blocks[idx(x, ny, z)] = Block::Grass as u8;
                height[(z * SX + x) as usize] = ny;
            }
        }
        w.nest = ivec3(nx, ny + 1, nz);

        // Trees.
        let mut trees: Vec<(i32, i32)> = Vec::new();
        for _ in 0..900 {
            let x = rng.irange(3, SX - 3);
            let z = rng.irange(3, SZ - 3);
            if (x - nx).abs() < 8 && (z - nz).abs() < 8 {
                continue;
            }
            let h = height[(z * SX + x) as usize];
            if w.get(x, h, z) != Block::Grass
                || trees
                    .iter()
                    .any(|&(tx, tz)| (tx - x).abs() + (tz - z).abs() < 5)
            {
                continue;
            }
            if fbm2(seed ^ 0x7777, x as f32 / 30.0, z as f32 / 30.0, 2) < 0.45 && trees.len() > 40 {
                continue; // clearings
            }
            trees.push((x, z));
            let trunk = rng.irange(4, 7);
            for y in h + 1..=h + trunk {
                w.blocks[idx(x, y, z)] = Block::Log as u8;
            }
            w.blocks[idx(x, h, z)] = Block::Dirt as u8;
            let top = h + trunk;
            for dy in -2..=1 {
                let r: i32 = if dy >= 0 { 1 } else { 2 };
                for dz in -r..=r {
                    for dx in -r..=r {
                        if dx.abs() == r && dz.abs() == r && (dy == 1 || rng.chance(0.5)) {
                            continue;
                        }
                        let (lx, ly, lz) = (x + dx, top + dy, z + dz);
                        if in_bounds(lx, ly, lz) && w.get(lx, ly, lz) == Block::Air {
                            w.blocks[idx(lx, ly, lz)] = Block::Leaves as u8;
                        }
                    }
                }
            }
            if in_bounds(x, top + 2, z) {
                w.blocks[idx(x, top + 2, z)] = Block::Leaves as u8;
            }
        }

        w.place_landmarks(&mut rng, &height);
        w.initial_light();
        w
    }

    /// Picks spots for shards, wild sprouts and chompers and builds the shard challenges.
    fn place_landmarks(&mut self, rng: &mut Rng, height: &[i32]) {
        let nest = self.nest;
        let hgt = |x: i32, z: i32| height[(z * SX + x) as usize];
        let far_enough =
            |x: i32, z: i32, min: i32| (x - nest.x).abs().max((z - nest.z).abs()) >= min;

        // 1. A shard on dry land guarded by chompers.
        // 2. A shard on an island in a lake (blue sprouts or a bridge).
        // 3. A shard in a lava moat (red sprouts or a bridge).
        // 4. A shard buried in a stone vault underground (dig!).
        // 5. A shard on a high hill top.
        let mut used: Vec<IVec3> = Vec::new();
        let ok_spacing = |p: IVec3, used: &Vec<IVec3>| {
            used.iter()
                .all(|u| (u.x - p.x).abs() + (u.z - p.z).abs() > 28)
        };

        // Hill top: highest land point far from the nest.
        let mut best = ivec3(10, 0, 10);
        for z in (6..SZ - 6).step_by(2) {
            for x in (6..SX - 6).step_by(2) {
                if far_enough(x, z, 30) && hgt(x, z) > best.y {
                    best = ivec3(x, hgt(x, z), z);
                }
            }
        }
        let hill = ivec3(best.x, self.surface_y(best.x, best.z) + 1, best.z);
        for y in hill.y..SY {
            for (dx, dz) in [(0, 0), (1, 0), (0, 1), (-1, 0), (0, -1)] {
                let (x, z) = (hill.x + dx, hill.z + dz);
                if self.get(x, y, z) != Block::Air {
                    self.blocks[idx(x, y, z)] = Block::Air as u8;
                }
            }
        }
        used.push(hill);

        // Lake island: find a deep water spot.
        let mut lake = None;
        for _ in 0..4000 {
            let (x, z) = (rng.irange(10, SX - 10), rng.irange(10, SZ - 10));
            let h = hgt(x, z);
            let p = ivec3(x, SEA + 1, z);
            if h < SEA - 2 && far_enough(x, z, 25) && ok_spacing(p, &used) {
                lake = Some(p);
                break;
            }
        }
        let lake = lake.unwrap_or_else(|| {
            // No natural lake: dig one.
            let (x, z) = (nest.x + 34, nest.z - 30);
            for dz in -7..=7 {
                for dx in -7..=7 {
                    if dx * dx + dz * dz <= 49 {
                        for y in SEA - 3..SY {
                            let b = if y <= SEA { Block::Water } else { Block::Air };
                            self.blocks[idx(x + dx, y, z + dz)] = b as u8;
                        }
                    }
                }
            }
            ivec3(x, SEA + 1, z)
        });
        for dz in -1..=1 {
            for dx in -1..=1 {
                for y in 1..=SEA {
                    let b = if y == SEA { Block::Sand } else { Block::Stone };
                    self.blocks[idx(lake.x + dx, y, lake.z + dz)] = b as u8;
                }
                for y in SEA + 1..SY {
                    self.blocks[idx(lake.x + dx, y, lake.z + dz)] = Block::Air as u8;
                }
            }
        }
        used.push(lake);

        // Lava moat on dry land.
        let mut moat = ivec3(nest.x - 40, 0, nest.z + 36);
        for _ in 0..4000 {
            let (x, z) = (rng.irange(12, SX - 12), rng.irange(12, SZ - 12));
            let p = ivec3(x, hgt(x, z), z);
            if p.y > SEA + 2 && p.y < 40 && far_enough(x, z, 26) && ok_spacing(p, &used) {
                moat = p;
                break;
            }
        }
        let my = moat.y;
        for dz in -4..=4i32 {
            for dx in -4..=4i32 {
                let (x, z) = (moat.x + dx, moat.z + dz);
                let ring = dx.abs().max(dz.abs());
                for y in my + 1..SY {
                    self.blocks[idx(x, y, z)] = Block::Air as u8;
                }
                for y in my - 3..my {
                    self.blocks[idx(x, y, z)] = Block::Stone as u8;
                }
                let top = match ring {
                    0 | 1 => Block::Grass,
                    2 | 3 => Block::Lava,
                    _ => Block::Stone,
                };
                self.blocks[idx(x, my, z)] = top as u8;
                if top == Block::Lava {
                    self.blocks[idx(x, my - 1, z)] = Block::Lava as u8;
                }
            }
        }
        // Island stands one block proud so the lava cannot flow (it does not flow anyway).
        let moat = ivec3(moat.x, my + 1, moat.z);
        used.push(moat);

        // Underground vault.
        let mut vault = ivec3(nest.x + 30, 0, nest.z + 30);
        for _ in 0..4000 {
            let (x, z) = (rng.irange(12, SX - 12), rng.irange(12, SZ - 12));
            let h = hgt(x, z);
            let p = ivec3(x, h, z);
            if h > SEA + 6 && far_enough(x, z, 20) && ok_spacing(p, &used) {
                vault = p;
                break;
            }
        }
        let vy = (vault.y - 9).max(6);
        for dz in -2..=2i32 {
            for dx in -2..=2i32 {
                for dy in -1..=3i32 {
                    let edge = dx.abs() == 2 || dz.abs() == 2 || dy == -1 || dy == 3;
                    let b = if edge { Block::Stone } else { Block::Air };
                    self.blocks[idx(vault.x + dx, vy + dy, vault.z + dz)] = b as u8;
                }
            }
        }
        // A crystal "chimney" marks the spot on the surface.
        for y in vy + 4..=vault.y {
            self.blocks[idx(vault.x, y, vault.z)] = Block::CrystalOre as u8;
        }
        let vault = ivec3(vault.x, vy, vault.z);
        used.push(vault);

        // Guarded meadow shard.
        let mut meadow = ivec3(nest.x - 35, 0, nest.z - 35);
        for _ in 0..4000 {
            let (x, z) = (rng.irange(10, SX - 10), rng.irange(10, SZ - 10));
            let p = ivec3(x, hgt(x, z) + 1, z);
            if p.y > SEA + 2
                && far_enough(x, z, 22)
                && ok_spacing(p, &used)
                && self.get(x, p.y, z) == Block::Air
            {
                meadow = p;
                break;
            }
        }
        meadow.y = self.surface_y(meadow.x, meadow.z) + 1;
        used.push(meadow);

        self.shard_spots = vec![meadow, hill, moat, lake, vault];
        // Guards sleep on firm ground a few steps from three of the shards.
        for s in [meadow, hill, moat] {
            'spot: for r in 3..9 {
                for k in 0..12 {
                    let a = k as f32 / 12.0 * std::f32::consts::TAU;
                    let (x, z) = (
                        s.x + (a.cos() * r as f32) as i32,
                        s.z + (a.sin() * r as f32) as i32,
                    );
                    if x < 1 || z < 1 || x >= SX - 1 || z >= SZ - 1 {
                        continue;
                    }
                    let y = self.surface_y(x, z);
                    let top = self.get(x, y, z);
                    if top.is_opaque()
                        && self.get(x, y + 1, z) == Block::Air
                        && (y - s.y).abs() <= 3
                    {
                        self.chomper_spots.push(ivec3(x, y + 1, z));
                        break 'spot;
                    }
                }
            }
        }

        // Wild sprouts: red near the nest, yellow and blue scattered elsewhere.
        for (dx, dz) in [(3, 2), (-3, 2), (2, -3)] {
            self.sprout_spots
                .push((ivec3(nest.x + dx, nest.y, nest.z + dz), 0));
        }
        let mut placed = 0;
        for _ in 0..5000 {
            if placed >= 18 {
                break;
            }
            let (x, z) = (rng.irange(4, SX - 4), rng.irange(4, SZ - 4));
            let y = self.surface_y(x, z);
            if y <= SEA || self.get(x, y, z) != Block::Grass || !far_enough(x, z, 12) {
                continue;
            }
            let color = if placed % 2 == 0 { 1 } else { 2 };
            let count = rng.irange(2, 4);
            for k in 0..count {
                let (sx, sz) = (x + k, z + (k % 2));
                let sy = self.surface_y(sx, sz);
                if self.get(sx, sy, sz) == Block::Grass {
                    self.sprout_spots.push((ivec3(sx, sy + 1, sz), color));
                }
            }
            placed += 1;
        }
        // A few more sleeping chompers out in the wild.
        for _ in 0..3000 {
            if self.chomper_spots.len() >= 9 {
                break;
            }
            let (x, z) = (rng.irange(6, SX - 6), rng.irange(6, SZ - 6));
            let y = self.surface_y(x, z);
            if y > SEA && far_enough(x, z, 30) && self.get(x, y, z) == Block::Grass {
                self.chomper_spots.push(ivec3(x, y + 1, z));
            }
        }
    }

    // ------------------------------------------------------------------ queries

    /// Voxel DDA ray cast that stops at solid blocks.
    pub fn raycast(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<RayHit> {
        let dir = dir.normalize_or_zero();
        if dir == Vec3::ZERO {
            return None;
        }
        let mut p = ivec3(
            origin.x.floor() as i32,
            origin.y.floor() as i32,
            origin.z.floor() as i32,
        );
        let step = ivec3(
            dir.x.signum() as i32,
            dir.y.signum() as i32,
            dir.z.signum() as i32,
        );
        let inv = vec3(1.0 / dir.x.abs(), 1.0 / dir.y.abs(), 1.0 / dir.z.abs());
        let frac = |o: f32, s: i32| {
            if s > 0 {
                o.floor() + 1.0 - o
            } else {
                o - o.floor()
            }
        };
        let mut t_max = vec3(
            frac(origin.x, step.x) * inv.x,
            frac(origin.y, step.y) * inv.y,
            frac(origin.z, step.z) * inv.z,
        );
        let mut normal = IVec3::ZERO;
        let mut dist = 0.0;
        while dist <= max_dist {
            if p.y >= 0 && p.y < SY && in_bounds(p.x, p.y, p.z) && self.get_v(p).is_solid() {
                return Some(RayHit {
                    pos: p,
                    normal,
                    dist,
                });
            }
            if t_max.x < t_max.y && t_max.x < t_max.z {
                p.x += step.x;
                dist = t_max.x;
                t_max.x += inv.x;
                normal = ivec3(-step.x, 0, 0);
            } else if t_max.y < t_max.z {
                p.y += step.y;
                dist = t_max.y;
                t_max.y += inv.y;
                normal = ivec3(0, -step.y, 0);
            } else {
                p.z += step.z;
                dist = t_max.z;
                t_max.z += inv.z;
                normal = ivec3(0, 0, -step.z);
            }
        }
        None
    }

    pub fn is_exposed(&self, p: IVec3) -> bool {
        DIRS.iter().any(|d| !self.get_v(p + *d).is_opaque())
    }

    fn passable(&self, p: IVec3, opts: &PathOpts) -> Option<u32> {
        // Returns the extra cost of making the cell passable (0 = already open).
        let b = self.get_v(p);
        if !in_bounds(p.x, p.y, p.z) {
            return None;
        }
        match b {
            Block::Air => Some(0),
            Block::Water => Some(if opts.swim { 0 } else { 6 }),
            Block::Lava => {
                if opts.fireproof {
                    Some(1)
                } else {
                    None
                }
            }
            _ => match b.hardness() {
                Some(h) if opts.dig => Some(4 + (h * 2.0) as u32),
                _ => None,
            },
        }
    }

    /// A* over standing positions (feet cells). Moves may step up one block, drop up to
    /// three, swim, and (optionally) dig through blocks at a cost.
    pub fn find_path(
        &self,
        start: IVec3,
        goal: IVec3,
        goal_radius: i32,
        opts: PathOpts,
    ) -> Option<Vec<IVec3>> {
        #[derive(PartialEq, Eq)]
        struct Node(u32, IVec3);
        impl Ord for Node {
            fn cmp(&self, o: &Self) -> std::cmp::Ordering {
                o.0.cmp(&self.0)
            }
        }
        impl PartialOrd for Node {
            fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(o))
            }
        }
        let h = |p: IVec3| {
            let d = (p - goal).abs();
            ((d.x + d.z) * 10 + d.y * 6) as u32
        };
        let at_goal = |p: IVec3| {
            let d = (p - goal).abs();
            d.x.max(d.z) <= goal_radius && d.y <= 2
        };
        let mut open = BinaryHeap::new();
        let mut came: HashMap<IVec3, IVec3> = HashMap::new();
        let mut cost: HashMap<IVec3, u32> = HashMap::new();
        open.push(Node(h(start), start));
        cost.insert(start, 0);
        let mut expanded = 0;
        let mut best = (u32::MAX, start);
        while let Some(Node(_, p)) = open.pop() {
            if at_goal(p) {
                best = (0, p);
                break;
            }
            expanded += 1;
            if expanded > opts.max_nodes {
                break;
            }
            let hp = h(p);
            if hp < best.0 {
                best = (hp, p);
            }
            let base = cost[&p];
            for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                for dy in [0, 1, -1, -2, -3] {
                    let n = p + ivec3(dx, dy, dz);
                    if !in_bounds(n.x, n.y, n.z) || n.y < 1 {
                        continue;
                    }
                    let mut extra = 0;
                    let mut cells = vec![n, n + IVec3::Y];
                    if dy > 0 {
                        cells.push(p + IVec3::Y * 2);
                    }
                    if dy < 0 {
                        for k in 0..(-dy) {
                            cells.push(n + IVec3::Y * (2 + k));
                        }
                    }
                    let mut ok = true;
                    let mut digging = false;
                    for c in &cells {
                        digging |= self.get_v(*c).is_solid();
                        match self.passable(*c, &opts) {
                            Some(c) => extra += c,
                            None => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if !ok {
                        continue;
                    }
                    // Needs something to stand on (or water to swim in).
                    let below = self.get_v(n - IVec3::Y);
                    let feet = self.get_v(n);
                    let supported = below.is_solid() || feet.is_liquid();
                    if !supported {
                        continue;
                    }
                    if dy < 0 && digging {
                        continue; // never dig while dropping
                    }
                    let step = 10 + extra * 10 + if dy > 0 { 4 } else { 0 };
                    let nc = base + step;
                    if cost.get(&n).map_or(true, |&c| nc < c) {
                        cost.insert(n, nc);
                        came.insert(n, p);
                        open.push(Node(nc + h(n), n));
                    }
                }
            }
        }
        if best.0 != 0 && !opts.partial {
            return None;
        }
        let mut path = vec![best.1];
        let mut cur = best.1;
        while let Some(&prev) = came.get(&cur) {
            path.push(prev);
            cur = prev;
        }
        path.reverse();
        Some(path)
    }
}

#[derive(Clone, Copy)]
pub struct PathOpts {
    pub swim: bool,
    pub fireproof: bool,
    pub dig: bool,
    pub partial: bool,
    pub max_nodes: usize,
}

#[allow(dead_code)]
pub fn chunk_of(x: i32, z: i32) -> (i32, i32) {
    (x.div_euclid(CHUNK), z.div_euclid(CHUNK))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_landmarks() {
        let w = World::generate(1234);
        assert_eq!(w.shard_spots.len(), 5);
        assert!(w.get_v(w.nest - IVec3::Y).is_solid());
        assert_eq!(w.get_v(w.nest), Block::Air);
        assert_eq!(w.sky_light(w.nest.x, w.nest.y, w.nest.z), 15);
    }

    #[test]
    fn lighting_updates_incrementally() {
        let mut w = World::generate(99);
        let p = w.nest + ivec3(2, 0, 2);
        assert_eq!(w.sky_light(p.x, p.y, p.z), 15);
        // Roof over the cell darkens it but leaves some light from the sides.
        w.set(p + IVec3::Y, Block::Stone);
        let l = w.sky_light(p.x, p.y, p.z);
        assert!(l < 15 && l > 10, "light {l}");
        w.set(p + IVec3::Y, Block::Air);
        assert_eq!(w.sky_light(p.x, p.y, p.z), 15);
        w.set(p, Block::GlowLamp);
        assert_eq!(w.block_light(p.x + 1, p.y, p.z), 14);
        w.set(p, Block::Air);
        assert_eq!(w.block_light(p.x + 1, p.y, p.z), 0);
    }

    #[test]
    fn paths_reach_nest_from_every_shard() {
        for seed in [1u32, 7, 42, 2024] {
            let w = World::generate(seed);
            for s in &w.shard_spots {
                let opts = PathOpts {
                    swim: true,
                    fireproof: true,
                    dig: true,
                    partial: false,
                    max_nodes: 60_000,
                };
                let path = w.find_path(*s, w.nest, 2, opts);
                assert!(path.is_some(), "seed {seed}: no path from {s:?}");
            }
        }
    }

    #[test]
    fn raycast_hits_ground() {
        let w = World::generate(5);
        let o = w.nest.as_vec3() + vec3(0.5, 3.0, 0.5);
        let hit = w.raycast(o, vec3(0.0, -1.0, 0.0), 10.0).expect("hit");
        assert_eq!(hit.pos, w.nest - IVec3::Y);
        assert_eq!(hit.normal, IVec3::Y);
    }
}
