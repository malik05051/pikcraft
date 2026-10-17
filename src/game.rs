//! Game state and simulation: the player, the sprout squad, chompers, carried items and the nest.

use crate::block::{ALL_BLOCKS, Block};
use crate::entities::*;
use crate::noise::Rng;
use crate::physics::{Body, GRAVITY};
use crate::world::{PathOpts, SEA, SX, SY, SZ, World};
use macroquad::math::{IVec3, Vec3, ivec3, vec3};
use std::collections::HashMap;

pub const DAY_LENGTH: f32 = 480.0;
pub const MAX_SPROUTS: usize = 120;
pub const TOTAL_SHARDS: u32 = 5;
pub const HOTBAR: [Option<Block>; 9] = [
    None,
    Some(Block::Dirt),
    Some(Block::Stone),
    Some(Block::Planks),
    Some(Block::Log),
    Some(Block::Bricks),
    Some(Block::Glass),
    Some(Block::GlowLamp),
    Some(Block::Sand),
];
pub const TOOL_NAMES: [&str; 4] = [
    "Bare hands",
    "Wood Pickaxe",
    "Stone Pickaxe",
    "Iron Pickaxe",
];

/// Everything the player can do this frame, decoupled from the keyboard so demos can drive it.
#[derive(Default, Clone)]
pub struct Input {
    pub forward: f32,
    pub strafe: f32,
    pub jump: bool,
    pub sprint: bool,
    pub look: (f32, f32),
    pub primary: bool,
    pub primary_pressed: bool,
    pub secondary: bool,
    pub secondary_pressed: bool,
    pub throw: bool,
    pub whistle: bool,
    pub pluck: bool,
    pub dismiss: bool,
    pub cycle_color: bool,
    pub slot: Option<usize>,
    pub scroll: f32,
}

pub struct Recipe {
    pub name: &'static str,
    pub needs: &'static [(Block, u32)],
    pub output: Output,
}

pub enum Output {
    Blocks(Block, u32),
    Tool(u8),
    Seed,
}

pub const RECIPES: [Recipe; 8] = [
    Recipe {
        name: "4 Planks",
        needs: &[(Block::Log, 1)],
        output: Output::Blocks(Block::Planks, 4),
    },
    Recipe {
        name: "Wood Pickaxe",
        needs: &[(Block::Planks, 3), (Block::Log, 2)],
        output: Output::Tool(1),
    },
    Recipe {
        name: "Stone Pickaxe",
        needs: &[(Block::Stone, 3), (Block::Log, 2)],
        output: Output::Tool(2),
    },
    Recipe {
        name: "Iron Pickaxe",
        needs: &[(Block::IronOre, 3), (Block::Log, 2), (Block::CoalOre, 1)],
        output: Output::Tool(3),
    },
    Recipe {
        name: "2 Glass",
        needs: &[(Block::Sand, 2), (Block::CoalOre, 1)],
        output: Output::Blocks(Block::Glass, 2),
    },
    Recipe {
        name: "4 Bricks",
        needs: &[(Block::Stone, 4)],
        output: Output::Blocks(Block::Bricks, 4),
    },
    Recipe {
        name: "3 Glow Lamps",
        needs: &[(Block::CrystalOre, 1), (Block::Planks, 1)],
        output: Output::Blocks(Block::GlowLamp, 3),
    },
    Recipe {
        name: "Sprout Seed",
        needs: &[(Block::Leaves, 4), (Block::CrystalOre, 1)],
        output: Output::Seed,
    },
];

pub struct Player {
    pub body: Body,
    pub yaw: f32,
    pub pitch: f32,
    pub hp: f32,
    pub hurt: f32,
    pub since_hurt: f32,
    pub inv: [u32; ALL_BLOCKS.len()],
    pub tool: u8,
    pub slot: usize,
    pub dig_target: Option<IVec3>,
    pub punch_cd: f32,
    pub place_cd: f32,
    pub throw_cd: f32,
    pub pluck_cd: f32,
    pub throw_color: u8,
    pub whistle: f32,
    pub whistle_center: Vec3,
    pub air: f32,
    pub swing: f32,
}

impl Player {
    pub fn eye(&self) -> Vec3 {
        self.body.pos + vec3(0.0, 1.62, 0.0)
    }

    pub fn forward(&self) -> Vec3 {
        vec3(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
    }

    pub fn flat_forward(&self) -> Vec3 {
        vec3(self.yaw.cos(), 0.0, self.yaw.sin())
    }

    pub fn right(&self) -> Vec3 {
        vec3(-self.yaw.sin(), 0.0, self.yaw.cos())
    }

    pub fn has(&self, b: Block) -> u32 {
        self.inv[b as usize]
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Target {
    None,
    Block {
        pos: IVec3,
        normal: IVec3,
        point: Vec3,
        dist: f32,
    },
    Enemy {
        id: u32,
        point: Vec3,
        dist: f32,
    },
    Item {
        id: u32,
        point: Vec3,
        dist: f32,
    },
    Sprout {
        id: u32,
        point: Vec3,
        dist: f32,
    },
}

impl Target {
    pub fn dist(&self) -> f32 {
        match *self {
            Target::None => f32::INFINITY,
            Target::Block { dist, .. }
            | Target::Enemy { dist, .. }
            | Target::Item { dist, .. }
            | Target::Sprout { dist, .. } => dist,
        }
    }
}

pub struct Toast {
    pub text: String,
    pub time: f32,
    pub key: Option<Block>,
    pub count: u32,
}

#[derive(Default)]
pub struct Stats {
    pub plucked: u32,
    pub lost: u32,
    pub chompers: u32,
    pub delivered: u32,
}

pub struct Game {
    pub world: World,
    pub player: Player,
    pub sprouts: Vec<Sprout>,
    pub enemies: Vec<Enemy>,
    pub items: Vec<Item>,
    pub particles: Vec<Particle>,
    pub dig: HashMap<IVec3, f32>,
    pub rng: Rng,
    pub next_id: u32,
    pub time_of_day: f32,
    pub day: u32,
    pub elapsed: f32,
    pub shards: u32,
    pub won_at: Option<f32>,
    pub toasts: Vec<Toast>,
    pub nest: Vec3,
    pub nest_pulse: f32,
    pub spawn_timer: f32,
    pub stats: Stats,
    pub was_night: bool,
}

fn yaw_to(from: Vec3, to: Vec3) -> f32 {
    (to.z - from.z).atan2(to.x - from.x)
}

fn ray_sphere(o: Vec3, d: Vec3, c: Vec3, r: f32) -> Option<f32> {
    let oc = o - c;
    let b = oc.dot(d);
    let cc = oc.dot(oc) - r * r;
    let disc = b * b - cc;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t >= 0.0 {
        Some(t)
    } else if cc < 0.0 {
        Some(0.0)
    } else {
        None
    }
}

impl Game {
    pub fn new(seed: u32) -> Game {
        let world = World::generate(seed);
        let nest = world.nest.as_vec3() + vec3(0.5, 0.0, 0.5);
        let spawn = nest + vec3(0.0, 0.0, -3.5);
        let mut g = Game {
            player: Player {
                body: Body::new(spawn, 0.3, 1.8),
                yaw: std::f32::consts::FRAC_PI_2,
                pitch: -0.25,
                hp: 20.0,
                hurt: 0.0,
                since_hurt: 99.0,
                inv: [0; ALL_BLOCKS.len()],
                tool: 0,
                slot: 0,
                dig_target: None,
                punch_cd: 0.0,
                place_cd: 0.0,
                throw_cd: 0.0,
                pluck_cd: 0.0,
                throw_color: C_RED,
                whistle: 0.0,
                whistle_center: spawn,
                air: 10.0,
                swing: 0.0,
            },
            sprouts: Vec::new(),
            enemies: Vec::new(),
            items: Vec::new(),
            particles: Vec::new(),
            dig: HashMap::new(),
            rng: Rng::new(seed as u64 ^ 0xBEEF),
            next_id: 1,
            time_of_day: 0.06,
            day: 1,
            elapsed: 0.0,
            shards: 0,
            won_at: None,
            toasts: Vec::new(),
            nest,
            nest_pulse: 0.0,
            spawn_timer: 30.0,
            stats: Stats::default(),
            was_night: false,
            world,
        };
        for (p, c) in g.world.sprout_spots.clone() {
            let id = g.new_id();
            g.sprouts.push(Sprout::new(
                id,
                c,
                p.as_vec3() + vec3(0.5, 0.0, 0.5),
                SproutState::Buried,
            ));
        }
        for (i, p) in g.world.chomper_spots.clone().into_iter().enumerate() {
            let id = g.new_id();
            let size = if i < 3 { 1.35 } else { 1.0 };
            let y = g.world.surface_y(p.x, p.z) + 1;
            g.enemies.push(Enemy::new(
                id,
                vec3(p.x as f32 + 0.5, y as f32, p.z as f32 + 0.5),
                size,
                false,
            ));
        }
        for p in g.world.shard_spots.clone() {
            let id = g.new_id();
            g.items.push(Item::new(
                id,
                ItemKind::Shard,
                p.as_vec3() + vec3(0.5, 0.0, 0.5),
            ));
        }
        g.toast("Welcome to PikCraft! Pluck the sprouts near the Nest with [E].");
        g
    }

    pub fn new_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    pub fn toast(&mut self, text: &str) {
        self.toasts.push(Toast {
            text: text.to_string(),
            time: 5.0,
            key: None,
            count: 0,
        });
        if self.toasts.len() > 6 {
            self.toasts.remove(0);
        }
    }

    fn toast_pickup(&mut self, b: Block, n: u32) {
        if let Some(t) = self.toasts.iter_mut().find(|t| t.key == Some(b)) {
            t.count += n;
            t.time = 3.0;
            t.text = format!("+{} {}", t.count, b.name());
            return;
        }
        self.toasts.push(Toast {
            text: format!("+{} {}", n, b.name()),
            time: 3.0,
            key: Some(b),
            count: n,
        });
        if self.toasts.len() > 6 {
            self.toasts.remove(0);
        }
    }

    // ------------------------------------------------------------------ time of day

    pub fn sun_height(&self) -> f32 {
        (self.time_of_day * std::f32::consts::TAU).sin()
    }

    pub fn daylight(&self) -> f32 {
        (self.sun_height() * 2.5 + 0.45).clamp(0.3, 1.0)
    }

    pub fn is_night(&self) -> bool {
        self.sun_height() < -0.08
    }

    pub fn sky_color(&self) -> [f32; 3] {
        let d = self.daylight();
        let day = [0.55, 0.75, 0.98];
        let night = [0.03, 0.04, 0.10];
        let dusk = [0.95, 0.55, 0.35];
        let s = self.sun_height();
        let dusk_k = (1.0 - (s.abs() * 4.0)).clamp(0.0, 1.0) * 0.6;
        let t = ((d - 0.3) / 0.7).clamp(0.0, 1.0);
        let mut c = [0.0; 3];
        for i in 0..3 {
            c[i] = night[i] + (day[i] - night[i]) * t;
            c[i] = c[i] + (dusk[i] - c[i]) * dusk_k;
        }
        c
    }

    /// Brightness multiplier for creatures standing at `p`.
    pub fn light_mul(&self, p: Vec3) -> f32 {
        let (sky, glow) = self.world.light_at(p + vec3(0.0, 0.3, 0.0));
        let l = (sky * self.daylight()).max(glow);
        0.82f32.powf((1.0 - l) * 15.0).max(0.05)
    }

    // ------------------------------------------------------------------ targeting

    /// What the crosshair points at, up to `reach` blocks.
    pub fn look_target(&self, reach: f32, include_sprouts: bool) -> Target {
        let o = self.player.eye();
        let d = self.player.forward();
        let mut best = Target::None;
        if let Some(hit) = self.world.raycast(o, d, reach) {
            best = Target::Block {
                pos: hit.pos,
                normal: hit.normal,
                point: o + d * hit.dist,
                dist: hit.dist,
            };
        }
        for e in self.enemies.iter().filter(|e| e.alive) {
            if let Some(t) = ray_sphere(o, d, e.body.center(), e.radius() + 0.15) {
                if t < best.dist() && t <= reach {
                    best = Target::Enemy {
                        id: e.id,
                        point: o + d * t,
                        dist: t,
                    };
                }
            }
        }
        for it in self.items.iter().filter(|i| i.alive) {
            let r = if matches!(it.kind, ItemKind::Block(_)) {
                0.35
            } else {
                0.7
            };
            if let Some(t) = ray_sphere(o, d, it.body.center(), r) {
                if t < best.dist() && t <= reach {
                    best = Target::Item {
                        id: it.id,
                        point: o + d * t,
                        dist: t,
                    };
                }
            }
        }
        if include_sprouts {
            for s in self
                .sprouts
                .iter()
                .filter(|s| s.state == SproutState::Buried)
            {
                if let Some(t) = ray_sphere(o, d, s.body.pos + vec3(0.0, 0.2, 0.0), 0.45) {
                    if t < best.dist() && t <= reach {
                        best = Target::Sprout {
                            id: s.id,
                            point: o + d * t,
                            dist: t,
                        };
                    }
                }
            }
        }
        best
    }

    fn mining_multiplier(&self, b: Block) -> f32 {
        let tool = self.player.tool as usize;
        if b.is_rocky() {
            [0.35, 1.4, 2.4, 4.0][tool]
        } else {
            [1.0, 1.4, 1.6, 1.8][tool]
        }
    }

    pub fn dig_progress(&self, p: IVec3) -> f32 {
        let h = self.world.get_v(p).hardness().unwrap_or(1.0);
        (self.dig.get(&p).copied().unwrap_or(0.0) / h).clamp(0.0, 1.0)
    }

    fn break_block(&mut self, p: IVec3, to_player: bool) {
        let b = self.world.get_v(p);
        if b.hardness().is_none() {
            return;
        }
        self.world.set(p, Block::Air);
        self.dig.remove(&p);
        let c = p.as_vec3() + vec3(0.5, 0.5, 0.5);
        self.burst(c, b.color(), 10, 3.0);
        let drop = b.drop();
        if drop == Block::Air {
            return;
        }
        if to_player {
            self.player.inv[drop as usize] += 1;
            self.toast_pickup(drop, 1);
        } else {
            let id = self.new_id();
            let mut it = Item::new(id, ItemKind::Block(drop), c - vec3(0.0, 0.3, 0.0));
            it.body.vel = vec3(self.rng.range(-1.0, 1.0), 3.0, self.rng.range(-1.0, 1.0));
            self.items.push(it);
        }
    }

    pub fn burst(&mut self, p: Vec3, color: [f32; 3], n: usize, speed: f32) {
        for _ in 0..n {
            let v = vec3(
                self.rng.range(-1.0, 1.0),
                self.rng.range(0.2, 1.5),
                self.rng.range(-1.0, 1.0),
            ) * speed;
            self.particles.push(Particle {
                pos: p + vec3(
                    self.rng.range(-0.3, 0.3),
                    self.rng.range(-0.3, 0.3),
                    self.rng.range(-0.3, 0.3),
                ),
                vel: v,
                color,
                life: self.rng.range(0.4, 0.9),
                size: self.rng.range(0.06, 0.12),
                gravity: 14.0,
            });
        }
    }

    fn spirit(&mut self, p: Vec3, color: [f32; 3]) {
        self.particles.push(Particle {
            pos: p + vec3(0.0, 0.5, 0.0),
            vel: vec3(0.0, 1.4, 0.0),
            color,
            life: 1.6,
            size: 0.16,
            gravity: -0.5,
        });
    }

    // ------------------------------------------------------------------ main update

    pub fn update(&mut self, input: &Input, dt: f32) {
        let dt = dt.min(0.05);
        self.elapsed += dt;
        self.time_of_day += dt / DAY_LENGTH;
        if self.time_of_day >= 1.0 {
            self.time_of_day -= 1.0;
            self.day += 1;
        }
        let night = self.is_night();
        if night && !self.was_night {
            self.toast("Night falls... chompers are on the prowl. Keep your sprouts close!");
        }
        if !night && self.was_night {
            self.toast(&format!(
                "Day {} dawns. Night chompers melt in the sunlight.",
                self.day
            ));
            for e in self.enemies.iter_mut().filter(|e| e.nocturnal && e.alive) {
                e.alive = false;
                e.hp = -1000.0;
            }
        }
        self.was_night = night;
        self.nest_pulse = (self.nest_pulse - dt).max(0.0);

        self.update_player(input, dt);
        self.update_sprouts(dt);
        self.update_enemies(dt);
        self.update_items(dt);
        self.update_particles(dt);
        self.spawn_night_chompers(dt);

        for t in &mut self.toasts {
            t.time -= dt;
        }
        self.toasts.retain(|t| t.time > 0.0);
        // Forget dig progress on blocks nobody is working on any more.
        if self.dig.len() > 64 {
            let w = &self.world;
            self.dig.retain(|p, _| w.get_v(*p).hardness().is_some());
        }
    }

    fn update_player(&mut self, input: &Input, dt: f32) {
        let p = &mut self.player;
        p.yaw += input.look.0;
        p.pitch = (p.pitch - input.look.1).clamp(-1.55, 1.55);
        if let Some(s) = input.slot {
            p.slot = s.min(8);
        }
        if input.scroll != 0.0 {
            let n = HOTBAR.len() as i32;
            p.slot = ((p.slot as i32 - input.scroll.signum() as i32).rem_euclid(n)) as usize;
        }
        for cd in [
            &mut p.punch_cd,
            &mut p.place_cd,
            &mut p.throw_cd,
            &mut p.pluck_cd,
        ] {
            *cd = (*cd - dt).max(0.0);
        }
        p.hurt = (p.hurt - dt).max(0.0);
        p.swing = (p.swing - dt * 4.0).max(0.0);
        p.since_hurt += dt;

        // Movement.
        let speed = if input.sprint { 6.2 } else { 4.4 };
        let mut wish = p.flat_forward() * input.forward + p.right() * input.strafe;
        if wish.length() > 1.0 {
            wish = wish.normalize();
        }
        let wish = wish * speed;
        let k = if p.body.on_ground { 14.0 } else { 4.0 };
        p.body.vel.x += (wish.x - p.body.vel.x) * (k * dt).min(1.0);
        p.body.vel.z += (wish.z - p.body.vel.z) * (k * dt).min(1.0);
        if input.jump {
            if p.body.in_water || p.body.in_lava {
                p.body.vel.y = p.body.vel.y.max(3.2);
            } else if p.body.on_ground {
                p.body.vel.y = 8.6;
            }
        }
        p.body.step(&self.world, dt, GRAVITY);
        // The nest is solid: push the player out of its trunk.
        let mut away = p.body.pos - self.nest;
        let dy = away.y;
        away.y = 0.0;
        let r = away.length();
        if r < 1.25 && dy > -1.0 && dy < 3.8 {
            let dir = if r > 1e-3 { away / r } else { Vec3::X };
            let out = self.nest + dir * 1.25;
            let np = vec3(out.x, p.body.pos.y, out.z);
            if !p.body.collides_at(&self.world, np) {
                p.body.pos = np;
            }
        }

        // Damage sources.
        let mut damage = 0.0;
        if p.body.landed_speed > 13.0 {
            damage += (p.body.landed_speed - 13.0) * 0.9;
            p.body.landed_speed = 0.0;
        }
        p.body.landed_speed = 0.0;
        if p.body.in_lava {
            damage += 6.0 * dt;
            p.body.vel.y = p.body.vel.y.max(4.0);
        }
        if p.body.head_in_water {
            p.air -= dt;
            if p.air < 0.0 {
                damage += 2.0 * dt;
            }
        } else {
            p.air = (p.air + dt * 3.0).min(10.0);
        }
        if p.since_hurt > 5.0 && p.hp < 20.0 {
            p.hp = (p.hp + dt / 3.0).min(20.0);
        }
        if damage > 0.0 {
            self.hurt_player(damage);
        }
        if self.player.body.pos.y < -10.0 {
            self.hurt_player(100.0);
        }

        // Squad commands.
        let p = &mut self.player;
        if input.cycle_color {
            let start = p.throw_color;
            for k in 1..=3 {
                let c = (start + k) % 3;
                if self
                    .sprouts
                    .iter()
                    .any(|s| s.color == c && s.state == SproutState::Follow)
                {
                    p.throw_color = c;
                    break;
                }
            }
        }
        let command_mode = HOTBAR[self.player.slot].is_none();
        let whistling = input.whistle || (command_mode && input.secondary);
        if whistling {
            let t = self.look_target(28.0, false);
            let p = &mut self.player;
            p.whistle_center = match t {
                Target::None => p.body.pos,
                Target::Block { pos, normal, .. } => (pos + normal).as_vec3() + vec3(0.5, 0.0, 0.5),
                Target::Enemy { point, .. }
                | Target::Item { point, .. }
                | Target::Sprout { point, .. } => point,
            };
            p.whistle = (p.whistle.max(1.0) + dt * 14.0).min(9.0);
            self.do_whistle();
        } else {
            self.player.whistle = 0.0;
        }
        if input.throw || (command_mode && input.primary_pressed) {
            self.try_throw();
        }
        if input.pluck && self.player.pluck_cd <= 0.0 {
            self.try_pluck();
        }
        if input.dismiss {
            self.dismiss();
        }

        if !command_mode {
            self.player_hands(input, dt);
        } else {
            self.player.dig_target = None;
        }
    }

    pub fn hurt_player(&mut self, dmg: f32) {
        let p = &mut self.player;
        p.hp -= dmg;
        p.hurt = 0.4;
        p.since_hurt = 0.0;
        if p.hp <= 0.0 {
            p.hp = 20.0;
            p.air = 10.0;
            p.body.pos = self.nest + vec3(0.0, 0.0, -3.5);
            p.body.vel = Vec3::ZERO;
            for s in &mut self.sprouts {
                if s.state == SproutState::Follow {
                    s.state = SproutState::Idle;
                }
            }
            self.toast("You fainted! You wake up back at the Nest.");
        }
    }

    fn player_hands(&mut self, input: &Input, dt: f32) {
        let target = self.look_target(5.0, true);
        if input.primary {
            match target {
                Target::Block { pos, .. } => {
                    let b = self.world.get_v(pos);
                    if let Some(h) = b.hardness() {
                        if self.player.dig_target != Some(pos) {
                            self.player.dig_target = Some(pos);
                        }
                        self.player.swing = 1.0;
                        let mult = self.mining_multiplier(b);
                        let prog = self.dig.entry(pos).or_insert(0.0);
                        *prog += dt * mult;
                        if *prog >= h {
                            self.break_block(pos, true);
                            self.player.dig_target = None;
                        }
                    }
                }
                Target::Enemy { id, .. } if self.player.punch_cd <= 0.0 => {
                    let dmg = [3.0, 5.0, 7.0, 9.0][self.player.tool as usize];
                    let fwd = self.player.flat_forward();
                    if let Some(e) = self.enemies.iter_mut().find(|e| e.id == id) {
                        e.hp -= dmg;
                        e.hurt = 0.2;
                        e.body.vel += fwd * 3.0 / e.size + vec3(0.0, 2.0, 0.0);
                        if e.state == EnemyState::Sleeping {
                            e.state = EnemyState::Chase;
                        }
                    }
                    self.player.punch_cd = 0.45;
                    self.player.swing = 1.0;
                }
                Target::Sprout { .. } if input.primary_pressed => self.try_pluck(),
                _ => {}
            }
        } else {
            self.player.dig_target = None;
        }
        // Undo progress on the block we stopped digging, as in classic block games.
        let keep = self.player.dig_target;
        if input.primary_pressed || !input.primary {
            self.dig.retain(|p, _| Some(*p) == keep);
        }

        if input.secondary && self.player.place_cd <= 0.0 {
            if let (Target::Block { pos, normal, .. }, Some(b)) = (target, HOTBAR[self.player.slot])
            {
                let at = pos + normal;
                let here = self.world.get_v(at);
                let free = matches!(here, Block::Air | Block::Water | Block::Lava);
                if free && self.player.has(b) > 0 && !self.blocks_entity(at) && at.y < SY {
                    self.world.set(at, b);
                    self.player.inv[b as usize] -= 1;
                    self.player.place_cd = 0.22;
                    self.player.swing = 1.0;
                }
            }
        }
    }

    fn blocks_entity(&self, at: IVec3) -> bool {
        let lo = at.as_vec3();
        let hi = lo + Vec3::ONE;
        let overlaps = |b: &Body| {
            b.pos.x + b.hw > lo.x
                && b.pos.x - b.hw < hi.x
                && b.pos.z + b.hw > lo.z
                && b.pos.z - b.hw < hi.z
                && b.pos.y + b.h > lo.y
                && b.pos.y < hi.y
        };
        overlaps(&self.player.body) || self.enemies.iter().any(|e| e.alive && overlaps(&e.body))
    }

    fn do_whistle(&mut self) {
        let c = self.player.whistle_center;
        let r = self.player.whistle;
        let ppos = self.player.body.pos;
        let mut recalled = 0;
        for s in &mut self.sprouts {
            if !s.in_field() || s.state == SproutState::Follow {
                continue;
            }
            let near_center = {
                let mut d = s.body.pos - c;
                d.y *= 0.5;
                d.length() <= r
            };
            if near_center || s.body.pos.distance(ppos) < 2.5 {
                if let SproutState::Carry(id) = s.state {
                    if let Some(it) = self.items.iter_mut().find(|i| i.id == id) {
                        it.carriers.retain(|c| *c != s.id);
                    }
                }
                if matches!(
                    s.state,
                    SproutState::Attack { latched: true, .. } | SproutState::Carry(_)
                ) {
                    s.body.vel = vec3(0.0, 3.0, 0.0);
                }
                s.state = SproutState::Follow;
                s.panic = 0.0;
                recalled += 1;
            }
        }
        let _ = recalled;
    }

    fn dismiss(&mut self) {
        let fwd = self.player.flat_forward();
        let right = self.player.right();
        let base = self.player.body.pos + fwd * 2.5;
        for s in &mut self.sprouts {
            if s.state == SproutState::Follow {
                s.state = SproutState::Idle;
                s.slot = base + right * ((s.color as f32 - 1.0) * 2.0);
                s.think = 1.0;
            }
        }
    }

    fn try_pluck(&mut self) {
        let ppos = self.player.body.pos;
        let looked = match self.look_target(4.0, true) {
            Target::Sprout { id, .. } => Some(id),
            _ => None,
        };
        let mut best: Option<(f32, usize)> = None;
        for (i, s) in self.sprouts.iter().enumerate() {
            if s.state != SproutState::Buried {
                continue;
            }
            let d = s.body.pos.distance(ppos);
            if Some(s.id) == looked || d < 2.6 {
                let score = if Some(s.id) == looked { -1.0 } else { d };
                if best.map_or(true, |(b, _)| score < b) {
                    best = Some((score, i));
                }
            }
        }
        if let Some((_, i)) = best {
            let s = &mut self.sprouts[i];
            s.state = SproutState::Follow;
            s.body.vel = vec3(0.0, 6.0, 0.0);
            s.body.pos.y += 0.2;
            let (pos, color) = (s.body.pos, COLOR_RGB[s.color as usize]);
            self.player.pluck_cd = 0.12;
            self.stats.plucked += 1;
            self.burst(pos, [0.5, 0.35, 0.2], 6, 2.0);
            self.burst(pos, color, 4, 2.0);
            if self.stats.plucked == 1 {
                self.toast("Plucked! Press [1] for the squad slot, then click (or [F]) to throw.");
            }
        }
    }

    fn try_throw(&mut self) {
        if self.player.throw_cd > 0.0 {
            return;
        }
        let ppos = self.player.body.pos;
        let mut color = self.player.throw_color;
        let squad_has = |c: u8, sprouts: &Vec<Sprout>| {
            sprouts.iter().any(|s| {
                s.color == c && s.state == SproutState::Follow && s.body.pos.distance(ppos) < 6.0
            })
        };
        if !squad_has(color, &self.sprouts) {
            match (0..3).find(|c| squad_has(*c, &self.sprouts)) {
                Some(c) => {
                    color = c;
                    self.player.throw_color = c;
                }
                None => return,
            }
        }
        let idx = self
            .sprouts
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                s.color == color
                    && s.state == SproutState::Follow
                    && s.body.pos.distance(ppos) < 6.0
            })
            .min_by(|a, b| {
                a.1.body
                    .pos
                    .distance(ppos)
                    .total_cmp(&b.1.body.pos.distance(ppos))
            })
            .map(|(i, _)| i);
        let Some(i) = idx else { return };

        let target = self.look_target(30.0, false);
        let (point, task) = match target {
            Target::None => {
                let p = self.player.eye() + self.player.forward() * 14.0;
                (vec3(p.x, p.y.min(self.player.eye().y), p.z), Task::None)
            }
            Target::Block {
                pos, normal, point, ..
            } => {
                let task = if self.world.get_v(pos).hardness().is_some() {
                    Task::Block(pos)
                } else {
                    Task::None
                };
                (point + normal.as_vec3() * 0.2, task)
            }
            Target::Enemy { id, point, .. } => (point, Task::Enemy(id)),
            Target::Item { id, point, .. } => (point, Task::Item(id)),
            Target::Sprout { point, .. } => (point, Task::None),
        };
        let start = self.player.eye() - vec3(0.0, 0.45, 0.0) + self.player.right() * 0.35;
        let dist = point.distance(start);
        let mut t = (dist / 15.0).clamp(0.28, 1.1);
        if color == C_YELLOW {
            t *= 1.3;
        }
        let d = point - start;
        let vel = vec3(d.x / t, d.y / t + 0.5 * GRAVITY * t, d.z / t);
        let s = &mut self.sprouts[i];
        s.body.pos = start - vec3(0.0, s.body.h * 0.5, 0.0);
        s.body.vel = vel;
        s.state = SproutState::Thrown(task);
        s.timer = 0.0;
        self.player.throw_cd = 0.18;
        self.player.swing = 1.0;
    }

    // ------------------------------------------------------------------ sprouts

    fn update_sprouts(&mut self, dt: f32) {
        let mut sprouts = std::mem::take(&mut self.sprouts);
        // Formation slots for followers.
        let mut follow_index = 0usize;
        let mut followers = Vec::new();
        for s in sprouts.iter() {
            if s.state == SproutState::Follow {
                followers.push(s.id);
            }
        }
        let fwd = self.player.flat_forward();
        let right = self.player.right();
        let ppos = self.player.body.pos;
        let squad_center = ppos - fwd * 1.8;

        let positions: Vec<(Vec3, bool)> =
            sprouts.iter().map(|s| (s.body.pos, s.in_field())).collect();

        for (si, s) in sprouts.iter_mut().enumerate() {
            if !s.alive() {
                continue;
            }
            s.anim += dt;
            if s.state == SproutState::Buried {
                continue;
            }
            s.age += dt;
            if s.maturity == 0 && s.age > 150.0 {
                s.maturity = 1;
            } else if s.maturity == 1 && s.age > 360.0 {
                s.maturity = 2;
            }
            s.timer += dt;
            s.think -= dt;

            let kinematic = matches!(
                s.state,
                SproutState::Carry(_) | SproutState::Attack { latched: true, .. }
            );
            match s.state {
                SproutState::Idle => {
                    let home = s.slot;
                    if home != Vec3::ZERO && home.distance(s.body.pos) > 0.8 && s.timer < 6.0 {
                        s.body
                            .steer(&self.world, home, s.speed() * 0.7, 10.0, dt, 8.6);
                    } else {
                        s.body.vel.x *= 1.0 - (8.0 * dt).min(1.0);
                        s.body.vel.z *= 1.0 - (8.0 * dt).min(1.0);
                    }
                    if s.think <= 0.0 {
                        s.think = 0.5;
                        if let Some(id) = self.find_carry_task(s.body.pos, 5.5) {
                            s.state = SproutState::GoCarry(id);
                            s.timer = 0.0;
                        }
                    }
                }
                SproutState::Follow => {
                    let k = followers
                        .iter()
                        .position(|id| *id == s.id)
                        .unwrap_or(follow_index) as f32;
                    follow_index += 1;
                    let ang = k * 2.39996;
                    let r = 0.45 * (k + 1.0).sqrt();
                    let target =
                        squad_center + right * (ang.cos() * r) - fwd * (ang.sin() * r).abs() * 0.8;
                    let d = target.distance(s.body.pos);
                    let speed = if d > 5.0 { s.speed() * 1.5 } else { s.speed() };
                    s.body.steer(&self.world, target, speed, 10.0, dt, 8.6);
                    s.yaw = yaw_to(s.body.pos, ppos);
                    if d > 70.0 {
                        s.state = SproutState::Idle;
                        s.slot = Vec3::ZERO;
                    }
                }
                SproutState::Thrown(task) => {
                    let c = s.body.center();
                    let mut latched = None;
                    for e in self.enemies.iter().filter(|e| e.alive) {
                        if c.distance(e.body.center()) < e.radius() + 0.45 {
                            latched = Some(e);
                            break;
                        }
                    }
                    if let Some(e) = latched {
                        let local = c - e.body.center();
                        let (sn, cs) = (-e.yaw).sin_cos();
                        s.slot = vec3(
                            local.x * cs - local.z * sn,
                            local.y,
                            local.x * sn + local.z * cs,
                        )
                        .normalize_or_zero()
                            * e.radius();
                        s.state = SproutState::Attack {
                            enemy: e.id,
                            latched: true,
                        };
                    } else if s.timer > 0.05
                        && (s.body.on_ground || s.body.in_water || s.body.in_lava)
                    {
                        s.state = self.land_task(task, s.body.pos);
                        s.timer = 0.0;
                        s.body.vel.x *= 0.2;
                        s.body.vel.z *= 0.2;
                    } else if s.timer > 4.0 {
                        s.state = SproutState::Idle;
                    }
                }
                SproutState::Mine(p) => {
                    let b = self.world.get_v(p);
                    if b.hardness().is_none() {
                        s.state = SproutState::Idle;
                        s.slot = s.body.pos;
                        s.timer = 0.0;
                    } else {
                        let c = s.body.center();
                        let lo = p.as_vec3();
                        let closest = c.clamp(lo, lo + Vec3::ONE);
                        let reach = closest.distance(c);
                        s.yaw = yaw_to(s.body.pos, lo + vec3(0.5, 0.0, 0.5));
                        if reach < 0.95 {
                            s.body.vel.x *= 0.5;
                            s.body.vel.z *= 0.5;
                            let prog = self.dig.entry(p).or_insert(0.0);
                            *prog += s.dig_power() * dt;
                            if *prog >= b.hardness().unwrap_or(1.0) {
                                self.break_block(p, false);
                                let next = self.next_vein_block(p, b, s.body.pos);
                                s.state = match next {
                                    Some(n) => SproutState::Mine(n),
                                    None => SproutState::Idle,
                                };
                                s.slot = s.body.pos;
                                s.timer = 0.0;
                                s.think = 0.3;
                            }
                        } else {
                            let goal = vec3(closest.x, s.body.pos.y, closest.z);
                            s.body.steer(&self.world, goal, s.speed(), 10.0, dt, 8.6);
                            if s.timer > 7.0 {
                                s.state = SproutState::Idle;
                                s.slot = s.body.pos;
                            }
                        }
                    }
                }
                SproutState::GoCarry(id) => {
                    match self.items.iter_mut().find(|i| i.id == id && i.alive) {
                        Some(it) if it.carriers.len() < it.max_carriers() => {
                            let d = it.body.pos.distance(s.body.pos);
                            if d < 0.9 {
                                it.carriers.push(s.id);
                                let n = it.carriers.len() as f32;
                                let ang = n * 2.39996;
                                let r = if matches!(it.kind, ItemKind::Block(_)) {
                                    0.35
                                } else {
                                    0.75
                                };
                                s.slot = vec3(ang.cos() * r, 0.0, ang.sin() * r);
                                s.state = SproutState::Carry(id);
                            } else {
                                s.body
                                    .steer(&self.world, it.body.pos, s.speed(), 10.0, dt, 8.6);
                                if s.timer > 10.0 {
                                    s.state = SproutState::Idle;
                                    s.slot = s.body.pos;
                                    s.think = 3.0;
                                }
                            }
                        }
                        _ => {
                            s.state = SproutState::Idle;
                            s.slot = s.body.pos;
                        }
                    }
                }
                SproutState::Carry(id) => match self.items.iter().find(|i| i.id == id && i.alive) {
                    Some(it) if it.carriers.contains(&s.id) => {
                        let target = it.body.pos + s.slot;
                        s.body.pos = s.body.pos.lerp(target, (12.0 * dt).min(1.0));
                        s.body.pos.y = it.body.pos.y;
                        s.body.vel = it.body.vel;
                        if it.body.vel.length() > 0.2 {
                            s.yaw = it.body.vel.z.atan2(it.body.vel.x);
                        } else {
                            s.yaw = yaw_to(s.body.pos, it.body.pos);
                        }
                    }
                    _ => {
                        s.state = SproutState::Idle;
                        s.slot = s.body.pos;
                    }
                },
                SproutState::Attack { enemy, latched } => {
                    match self.enemies.iter_mut().find(|e| e.id == enemy && e.alive) {
                        Some(e) => {
                            if latched {
                                let (sn, cs) = e.yaw.sin_cos();
                                let l = s.slot;
                                let world_off = vec3(l.x * cs - l.z * sn, l.y, l.x * sn + l.z * cs);
                                s.body.pos =
                                    e.body.center() + world_off - vec3(0.0, s.body.h * 0.5, 0.0);
                                s.body.vel = Vec3::ZERO;
                                s.yaw = yaw_to(s.body.pos, e.body.center());
                                e.hp -= s.attack_power() * dt;
                                e.hurt = 0.08;
                                if e.state == EnemyState::Sleeping {
                                    e.state = EnemyState::Chase;
                                }
                            } else {
                                let d = e.body.center().distance(s.body.center());
                                if d < e.radius() + 0.5 {
                                    let local = s.body.center() - e.body.center();
                                    let (sn, cs) = (-e.yaw).sin_cos();
                                    s.slot = vec3(
                                        local.x * cs - local.z * sn,
                                        local.y.max(0.0),
                                        local.x * sn + local.z * cs,
                                    )
                                    .normalize_or_zero()
                                        * e.radius();
                                    s.state = SproutState::Attack {
                                        enemy,
                                        latched: true,
                                    };
                                } else {
                                    s.body.steer(
                                        &self.world,
                                        e.body.pos,
                                        s.speed() * 1.1,
                                        10.0,
                                        dt,
                                        8.6,
                                    );
                                    if s.timer > 12.0 {
                                        s.state = SproutState::Idle;
                                    }
                                }
                            }
                        }
                        None => {
                            s.state = SproutState::Idle;
                            s.slot = Vec3::ZERO;
                            s.body.vel = vec3(0.0, 2.0, 0.0);
                        }
                    }
                }
                SproutState::Buried | SproutState::Dead => {}
            }

            // Personal space among loose sprouts.
            if matches!(s.state, SproutState::Follow | SproutState::Idle) {
                let mut push = Vec3::ZERO;
                for (oi, (op, of)) in positions.iter().enumerate() {
                    if oi == si || !*of {
                        continue;
                    }
                    let mut d = s.body.pos - *op;
                    d.y = 0.0;
                    let l = d.length();
                    if l < 0.4 && l > 1e-4 {
                        push += d / l * (0.4 - l);
                    }
                }
                s.body.vel += push * 20.0 * dt;
            }

            if kinematic {
                s.body.update_fluids(&self.world);
            } else {
                // Blue sprouts swim, the rest sink.
                if s.body.in_water && s.color == C_BLUE {
                    s.body.vel.y += 34.0 * dt;
                    s.body.vel.y = s.body.vel.y.min(2.5);
                }
                s.body.step(&self.world, dt, GRAVITY);
            }

            // Hazards.
            if s.color != C_BLUE && s.body.head_in_water {
                s.drown += dt;
            } else {
                s.drown = (s.drown - dt * 2.0).max(0.0);
            }
            if s.color != C_RED && s.body.in_lava {
                s.burn += dt;
            } else {
                s.burn = (s.burn - dt).max(0.0);
            }
            if s.drown > 3.0 || s.burn > 0.35 || s.body.pos.y < -5.0 {
                s.state = SproutState::Dead;
            }
        }

        // Deaths.
        let mut died = Vec::new();
        for s in &sprouts {
            if s.state == SproutState::Dead {
                died.push((
                    s.body.pos,
                    COLOR_RGB[s.color as usize],
                    s.drown > 3.0,
                    s.burn > 0.35,
                ));
            }
        }
        sprouts.retain(|s| s.alive());
        self.sprouts = sprouts;
        for (pos, color, drowned, burned) in died {
            self.stats.lost += 1;
            self.spirit(pos, color);
            if drowned {
                self.toast("A sprout drowned! Only blue sprouts can swim.");
            } else if burned {
                self.toast("A sprout burned up! Only red sprouts resist fire.");
            }
        }
    }

    /// Decide what a thrown sprout does once it lands.
    fn land_task(&self, task: Task, pos: Vec3) -> SproutState {
        match task {
            Task::Block(p) if self.world.get_v(p).hardness().is_some() => SproutState::Mine(p),
            Task::Enemy(id) if self.enemies.iter().any(|e| e.id == id && e.alive) => {
                SproutState::Attack {
                    enemy: id,
                    latched: false,
                }
            }
            Task::Item(id)
                if self
                    .items
                    .iter()
                    .any(|i| i.id == id && i.alive && i.carriers.len() < i.max_carriers()) =>
            {
                SproutState::GoCarry(id)
            }
            _ => {
                if let Some(id) = self.find_carry_task(pos, 3.0) {
                    SproutState::GoCarry(id)
                } else {
                    SproutState::Idle
                }
            }
        }
    }

    fn find_carry_task(&self, pos: Vec3, radius: f32) -> Option<u32> {
        self.items
            .iter()
            .filter(|i| i.alive && i.carriers.len() < i.max_carriers() && i.age > 0.4)
            .map(|i| (i.body.pos.distance(pos), i))
            .filter(|(d, _)| *d < radius)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, i)| i.id)
    }

    fn next_vein_block(&self, from: IVec3, kind: Block, near: Vec3) -> Option<IVec3> {
        let mut best: Option<(f32, IVec3)> = None;
        for dz in -2..=2 {
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let p = from + ivec3(dx, dy, dz);
                    if p == from {
                        continue;
                    }
                    let b = self.world.get_v(p);
                    let same = b == kind
                        || (kind == Block::Grass && b == Block::Dirt)
                        || (kind == Block::Dirt && b == Block::Grass);
                    if !same || !self.world.is_exposed(p) || b.hardness().is_none() {
                        continue;
                    }
                    let d = (p.as_vec3() + vec3(0.5, 0.5, 0.5)).distance(near);
                    if best.map_or(true, |(bd, _)| d < bd) {
                        best = Some((d, p));
                    }
                }
            }
        }
        best.filter(|(d, _)| *d < 3.5).map(|(_, p)| p)
    }

    // ------------------------------------------------------------------ chompers

    fn update_enemies(&mut self, dt: f32) {
        let mut enemies = std::mem::take(&mut self.enemies);
        let night = self.is_night();
        let ppos = self.player.body.pos;
        let mut latched: HashMap<u32, usize> = HashMap::new();
        for s in &self.sprouts {
            if let SproutState::Attack {
                enemy,
                latched: true,
            } = s.state
            {
                *latched.entry(enemy).or_insert(0) += 1;
            }
        }
        let mut player_dmg = 0.0;
        let mut eaten: Vec<u32> = Vec::new();
        let mut fling: Vec<u32> = Vec::new();
        let mut deaths = Vec::new();

        for e in enemies.iter_mut() {
            if !e.alive {
                continue;
            }
            e.hurt = (e.hurt - dt).max(0.0);
            e.bite_cd = (e.bite_cd - dt).max(0.0);
            e.shake_cd = (e.shake_cd - dt).max(0.0);
            e.timer += dt;
            if e.body.in_lava {
                e.hp -= 15.0 * dt;
            }
            if e.hp <= 0.0 {
                e.alive = false;
                deaths.push((e.body.pos, e.size));
                continue;
            }
            let n_latched = latched.get(&e.id).copied().unwrap_or(0);
            let aggro = if night { 15.0 } else { 9.0 } * e.size.sqrt();

            // Pick the nearest prey: the player or a loose sprout.
            let mut prey: Option<(f32, Vec3)> = None;
            let dp = e.body.pos.distance(ppos);
            if dp < aggro {
                prey = Some((dp, ppos));
            }
            for s in &self.sprouts {
                if !s.in_field() || matches!(s.state, SproutState::Attack { latched: true, .. }) {
                    continue;
                }
                let d = e.body.pos.distance(s.body.pos);
                if d < aggro && prey.map_or(true, |(b, _)| d < b) {
                    prey = Some((d, s.body.pos));
                }
            }

            match e.state {
                EnemyState::Sleeping => {
                    e.body.vel.x *= 0.8;
                    e.body.vel.z *= 0.8;
                    if night || dp < 2.5 * e.size || e.hp < e.max_hp {
                        e.state = EnemyState::Chase;
                        e.timer = 0.0;
                    }
                }
                EnemyState::Wander => {
                    if e.timer > 4.0 || e.wander_target.distance(e.body.pos) < 1.0 {
                        e.timer = 0.0;
                        let a = self.rng.range(0.0, std::f32::consts::TAU);
                        e.wander_target = e.body.pos + vec3(a.cos(), 0.0, a.sin()) * 6.0;
                    }
                    e.body
                        .steer(&self.world, e.wander_target, 1.4, 4.0, dt, 9.0);
                    e.yaw = turn_towards(e.yaw, yaw_to(e.body.pos, e.wander_target), 3.0 * dt);
                    if prey.is_some() {
                        e.state = EnemyState::Chase;
                        e.timer = 0.0;
                    } else if !night && !e.nocturnal && e.timer > 3.5 && self.rng.chance(0.3 * dt) {
                        e.state = EnemyState::Sleeping;
                    }
                }
                EnemyState::Chase => match prey {
                    Some((d, at)) => {
                        e.body.steer(
                            &self.world,
                            at,
                            2.7 + 0.4 * night as i32 as f32,
                            5.0,
                            dt,
                            9.0,
                        );
                        e.yaw = turn_towards(e.yaw, yaw_to(e.body.pos, at), 4.0 * dt);
                        if d < e.radius() + 1.0 && e.bite_cd <= 0.0 {
                            e.state = EnemyState::Bite;
                            e.timer = 0.0;
                        }
                        if n_latched as f32 >= 3.0 * e.size && e.shake_cd <= 0.0 {
                            e.state = EnemyState::Shake;
                            e.timer = 0.0;
                        }
                    }
                    None => {
                        e.state = EnemyState::Wander;
                        e.timer = 0.0;
                    }
                },
                EnemyState::Bite => {
                    e.body.vel.x *= 0.7;
                    e.body.vel.z *= 0.7;
                    e.mouth = (e.timer / 0.5).min(1.0);
                    if e.timer >= 0.55 {
                        let fwd = vec3(e.yaw.cos(), 0.0, e.yaw.sin());
                        let mouth = e.body.pos + fwd * (e.radius() + 0.35);
                        let reach = 1.0 * e.size;
                        let mut n = 0;
                        for s in &self.sprouts {
                            if n >= 3
                                || !s.in_field()
                                || matches!(s.state, SproutState::Attack { latched: true, .. })
                            {
                                continue;
                            }
                            if s.body.pos.distance(mouth) < reach {
                                eaten.push(s.id);
                                n += 1;
                            }
                        }
                        let mut pd = ppos - mouth;
                        pd.y = 0.0;
                        if pd.length() < reach + 0.3 && (ppos.y - e.body.pos.y).abs() < 1.5 * e.size
                        {
                            player_dmg += 4.0 * e.size;
                        }
                        e.mouth = 0.0;
                        e.bite_cd = 1.8;
                        e.state = EnemyState::Chase;
                        e.timer = 0.0;
                    }
                }
                EnemyState::Shake => {
                    e.body.vel.x = 0.0;
                    e.body.vel.z = 0.0;
                    e.yaw += dt * 14.0;
                    if e.timer > 0.8 {
                        for s in &self.sprouts {
                            if let SproutState::Attack {
                                enemy,
                                latched: true,
                            } = s.state
                            {
                                if enemy == e.id {
                                    fling.push(s.id);
                                }
                            }
                        }
                        e.shake_cd = 6.0;
                        e.state = EnemyState::Chase;
                        e.timer = 0.0;
                    }
                }
            }
            e.body.step(&self.world, dt, GRAVITY);
        }

        if !eaten.is_empty() {
            for s in &mut self.sprouts {
                if eaten.contains(&s.id) {
                    s.state = SproutState::Dead;
                }
            }
            let dead: Vec<(Vec3, u8)> = self
                .sprouts
                .iter()
                .filter(|s| s.state == SproutState::Dead)
                .map(|s| (s.body.pos, s.color))
                .collect();
            for (p, c) in dead {
                self.stats.lost += 1;
                self.spirit(p, COLOR_RGB[c as usize]);
            }
            self.sprouts.retain(|s| s.alive());
        }
        for s in &mut self.sprouts {
            if fling.contains(&s.id) {
                let a = self.rng.range(0.0, std::f32::consts::TAU);
                s.body.vel = vec3(a.cos() * 5.0, 6.0, a.sin() * 5.0);
                s.state = SproutState::Thrown(Task::None);
                s.timer = 0.0;
            }
        }
        if player_dmg > 0.0 {
            self.hurt_player(player_dmg);
        }
        // Turn the fallen into carcasses (night chompers that melted leave nothing).
        let mut keep = Vec::new();
        for e in enemies {
            if e.alive {
                keep.push(e);
            } else if e.hp <= -500.0 {
                self.burst(e.body.center(), [0.5, 0.3, 0.6], 20, 3.0);
            }
        }
        self.enemies = keep;
        for (pos, size) in deaths {
            self.stats.chompers += 1;
            let id = self.new_id();
            let seeds = (3.0 * size * size).round() as u32 + 1;
            self.items
                .push(Item::new(id, ItemKind::Carcass { seeds, size }, pos));
            self.burst(pos + vec3(0.0, 0.8, 0.0), [0.75, 0.35, 0.55], 24, 4.0);
            self.toast("Chomper defeated! Have sprouts carry it to the Nest for new seeds.");
        }
        // Sprouts latched to an enemy that no longer exists drop off.
        let ids: Vec<u32> = self.enemies.iter().map(|e| e.id).collect();
        for s in &mut self.sprouts {
            if let SproutState::Attack { enemy, .. } = s.state {
                if !ids.contains(&enemy) {
                    s.state = SproutState::Idle;
                    s.slot = Vec3::ZERO;
                    s.body.vel = vec3(0.0, 2.0, 0.0);
                }
            }
        }
    }

    fn spawn_night_chompers(&mut self, dt: f32) {
        if !self.is_night() {
            return;
        }
        self.spawn_timer -= dt;
        if self.spawn_timer > 0.0 {
            return;
        }
        self.spawn_timer = 14.0;
        let nocturnal = self.enemies.iter().filter(|e| e.nocturnal).count();
        if nocturnal >= 8 {
            return;
        }
        let p = self.player.body.pos;
        for _ in 0..30 {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let r = self.rng.range(20.0, 36.0);
            let (x, z) = ((p.x + a.cos() * r) as i32, (p.z + a.sin() * r) as i32);
            if x < 2 || z < 2 || x >= SX - 2 || z >= SZ - 2 {
                continue;
            }
            if (x as f32 - self.nest.x).abs() < 14.0 && (z as f32 - self.nest.z).abs() < 14.0 {
                continue;
            }
            let y = self.world.surface_y(x, z);
            if y <= SEA
                || !self.world.get(x, y, z).is_opaque()
                || self.world.get(x, y + 1, z) != Block::Air
            {
                continue;
            }
            let id = self.new_id();
            let size = if self.rng.chance(0.25) { 1.4 } else { 1.0 };
            self.enemies.push(Enemy::new(
                id,
                vec3(x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5),
                size,
                true,
            ));
            break;
        }
    }

    // ------------------------------------------------------------------ carried items

    fn update_items(&mut self, dt: f32) {
        let mut items = std::mem::take(&mut self.items);
        let ppos = self.player.body.pos;
        let mut delivered: Vec<(ItemKind, Vec<u32>)> = Vec::new();
        let mut picked: Vec<Block> = Vec::new();
        let mut dug: Vec<IVec3> = Vec::new();

        for it in items.iter_mut() {
            if !it.alive {
                continue;
            }
            it.age += dt;
            it.spin += dt;
            let iid = it.id;
            it.carriers.retain(|c| {
                self.sprouts
                    .iter()
                    .any(|s| s.id == *c && s.state == SproutState::Carry(iid))
            });
            let carriers: Vec<&Sprout> = self
                .sprouts
                .iter()
                .filter(|s| it.carriers.contains(&s.id))
                .collect();
            let lifting = !carriers.is_empty() && carriers.len() >= it.weight();

            if lifting {
                if it.repath <= 0.0 || it.path.is_empty() {
                    let opts = PathOpts {
                        swim: carriers.iter().all(|s| s.color == C_BLUE),
                        fireproof: carriers.iter().all(|s| s.color == C_RED),
                        dig: true,
                        partial: false,
                        max_nodes: 25_000,
                    };
                    let start = it.body.pos.floor().as_ivec3();
                    let goal = self.nest.floor().as_ivec3();
                    match self.world.find_path(start, goal, 1, opts) {
                        Some(p) => {
                            it.path = p;
                            it.path_i = 1.min(it.path.len() - 1);
                            it.no_path = false;
                            it.repath = 8.0;
                        }
                        None => {
                            it.path.clear();
                            it.no_path = true;
                            it.repath = 2.0;
                        }
                    }
                }
                it.repath -= dt;
                let power: f32 = carriers.iter().map(|s| s.dig_power()).sum();
                let ratio = (carriers.len() as f32 / it.weight() as f32).min(2.0);
                let mature: f32 =
                    carriers.iter().map(|s| s.speed()).sum::<f32>() / carriers.len() as f32 / 4.2;
                let speed = 1.5 * ratio.sqrt() * mature;
                if !it.path.is_empty() {
                    let wp = it.path[it.path_i.min(it.path.len() - 1)];
                    let wc = wp.as_vec3() + vec3(0.5, 0.0, 0.5);
                    let mut flat = wc - it.body.pos;
                    flat.y = 0.0;
                    if flat.length() < 0.3 && (wc.y - it.body.pos.y).abs() < 0.9 {
                        if it.path_i + 1 < it.path.len() {
                            it.path_i += 1;
                        }
                    }
                    // Dig anything blocking the next step.
                    let cur = it.body.pos.floor().as_ivec3();
                    let mut blockers = vec![wp, wp + IVec3::Y];
                    if wp.y > cur.y {
                        blockers.push(cur + IVec3::Y * 2);
                    }
                    if it.stuck > 3.0 {
                        // Can't climb it: tunnel straight towards the next waypoint instead.
                        let d = wp - cur;
                        let step = if d.x.abs() >= d.z.abs() {
                            ivec3(d.x.signum(), 0, 0)
                        } else {
                            ivec3(0, 0, d.z.signum())
                        };
                        blockers.push(cur + step);
                        blockers.push(cur + step + IVec3::Y);
                    }
                    let blocker = blockers
                        .into_iter()
                        .find(|b| self.world.get_v(*b).is_solid());
                    if let Some(b) = blocker {
                        it.body.vel.x *= 0.5;
                        it.body.vel.z *= 0.5;
                        match self.world.get_v(b).hardness() {
                            Some(h) => {
                                it.dig += power * dt;
                                if it.dig >= h {
                                    it.dig = 0.0;
                                    it.stuck = 0.0;
                                    dug.push(b);
                                }
                            }
                            None => {
                                it.repath = 0.0;
                                it.stuck = 0.0;
                            }
                        }
                    } else {
                        it.dig = 0.0;
                        it.body.steer(&self.world, wc, speed, 8.0, dt, 0.0);
                        let swimming = it.body.in_water || it.body.in_lava;
                        if wp.y > cur.y && (it.body.on_ground || swimming) && flat.length() < 1.2 {
                            it.body.vel.y = 8.8;
                        } else if swimming && wc.y + 0.2 > it.body.pos.y {
                            it.body.vel.y = it.body.vel.y.max(2.0);
                        }
                        // Only horizontal progress counts: hopping against a ledge is still stuck.
                        let horizontal = vec3(it.body.vel.x, 0.0, it.body.vel.z).length();
                        if horizontal < 0.25 {
                            it.stuck += dt;
                            if it.stuck > 2.0 && it.stuck - dt <= 2.0 {
                                it.repath = 0.0;
                            }
                        } else {
                            it.stuck = (it.stuck - dt).max(0.0);
                        }
                    }
                }
                // Arrived?
                let mut d = it.body.pos - self.nest;
                d.y = 0.0;
                if d.length() < 1.7 && (it.body.pos.y - self.nest.y).abs() < 3.0 {
                    it.alive = false;
                    delivered.push((it.kind, it.carriers.clone()));
                }
            } else {
                it.body.vel.x *= 1.0 - (6.0 * dt).min(1.0);
                it.body.vel.z *= 1.0 - (6.0 * dt).min(1.0);
                it.path.clear();
                it.repath = 0.0;
                it.dig = 0.0;
                // Loose blocks float in water and can be collected by walking over them.
                if it.body.in_water {
                    it.body.vel.y += 30.0 * dt;
                }
                if let ItemKind::Block(b) = it.kind {
                    if it.age > 0.6 && it.body.center().distance(ppos + vec3(0.0, 0.6, 0.0)) < 1.4 {
                        it.alive = false;
                        picked.push(b);
                    }
                    if it.age > 400.0 && it.carriers.is_empty() {
                        it.alive = false;
                    }
                }
            }
            it.body.step(&self.world, dt, GRAVITY);
        }
        items.retain(|i| i.alive);
        self.items = items;

        for p in dug {
            let b = self.world.get_v(p);
            self.world.set(p, Block::Air);
            self.burst(p.as_vec3() + vec3(0.5, 0.5, 0.5), b.color(), 6, 2.5);
        }
        for b in picked {
            self.player.inv[b as usize] += 1;
            self.toast_pickup(b, 1);
        }
        for (kind, carriers) in delivered {
            self.deliver(kind, &carriers);
        }
    }

    fn deliver(&mut self, kind: ItemKind, carriers: &[u32]) {
        self.nest_pulse = 1.0;
        self.stats.delivered += 1;
        let mut counts = [0u32; 3];
        for s in &mut self.sprouts {
            if carriers.contains(&s.id) {
                counts[s.color as usize] += 1;
                s.state = SproutState::Idle;
                s.slot = Vec3::ZERO;
                s.think = 1.0;
                s.body.vel = vec3(0.0, 3.0, 0.0);
            }
        }
        let top = self.nest + vec3(0.0, 3.2, 0.0);
        match kind {
            ItemKind::Block(b) => {
                self.player.inv[b as usize] += 1;
                self.toast_pickup(b, 1);
                self.burst(top, b.color(), 5, 2.0);
            }
            ItemKind::Carcass { seeds, .. } => {
                let color = (0..3).max_by_key(|c| counts[*c as usize]).unwrap_or(C_RED) as u8;
                let n = self.plant_seeds(color, seeds);
                self.burst(top, COLOR_RGB[color as usize], 30, 4.0);
                self.toast(&format!(
                    "The Nest sprouted {} {} seeds! Pluck them with [E].",
                    n, COLOR_NAMES[color as usize]
                ));
            }
            ItemKind::Shard => {
                self.shards += 1;
                self.burst(top, [1.0, 0.9, 0.3], 60, 6.0);
                self.toast(&format!(
                    "A Star Shard reached the Nest! ({}/{})",
                    self.shards, TOTAL_SHARDS
                ));
                if self.shards >= TOTAL_SHARDS && self.won_at.is_none() {
                    self.won_at = Some(self.elapsed);
                }
            }
        }
    }

    pub fn plant_seeds(&mut self, color: u8, n: u32) -> u32 {
        let mut planted = 0;
        for _ in 0..n {
            if self.sprouts.len() >= MAX_SPROUTS {
                self.toast("The field is full! (max 120 sprouts)");
                break;
            }
            for _ in 0..20 {
                let a = self.rng.range(0.0, std::f32::consts::TAU);
                let r = self.rng.range(2.2, 4.5);
                let (x, z) = (
                    (self.nest.x + a.cos() * r).floor() as i32,
                    (self.nest.z + a.sin() * r).floor() as i32,
                );
                let y = self.world.surface_y(x, z);
                if y < 0
                    || !self.world.get(x, y, z).is_solid()
                    || self.world.get(x, y + 1, z) != Block::Air
                {
                    continue;
                }
                let p = vec3(x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5);
                if self
                    .sprouts
                    .iter()
                    .any(|s| s.state == SproutState::Buried && s.body.pos.distance(p) < 0.6)
                {
                    continue;
                }
                let id = self.new_id();
                self.sprouts
                    .push(Sprout::new(id, color, p, SproutState::Buried));
                self.burst(p, COLOR_RGB[color as usize], 4, 1.5);
                planted += 1;
                break;
            }
        }
        planted
    }

    fn update_particles(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.vel.y -= p.gravity * dt;
            p.pos += p.vel * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        if self.particles.len() > 1500 {
            let n = self.particles.len() - 1500;
            self.particles.drain(0..n);
        }
    }

    // ------------------------------------------------------------------ crafting

    pub fn can_craft(&self, r: &Recipe) -> bool {
        if let Output::Tool(t) = r.output {
            if self.player.tool >= t {
                return false;
            }
        }
        r.needs.iter().all(|(b, n)| self.player.has(*b) >= *n)
    }

    pub fn craft(&mut self, i: usize) -> bool {
        let r = &RECIPES[i];
        if !self.can_craft(r) {
            return false;
        }
        for (b, n) in r.needs {
            self.player.inv[*b as usize] -= n;
        }
        match r.output {
            Output::Blocks(b, n) => {
                self.player.inv[b as usize] += n;
                self.toast(&format!("Crafted {}", r.name));
            }
            Output::Tool(t) => {
                self.player.tool = t;
                self.toast(&format!(
                    "Crafted a {}! Mining is faster.",
                    TOOL_NAMES[t as usize]
                ));
            }
            Output::Seed => {
                let c = self.player.throw_color;
                self.plant_seeds(c, 1);
                self.toast(&format!(
                    "A {} seed was planted by the Nest.",
                    COLOR_NAMES[c as usize]
                ));
            }
        }
        true
    }

    // ------------------------------------------------------------------ HUD helpers

    pub fn squad_counts(&self) -> ([u32; 3], [u32; 3]) {
        let mut squad = [0; 3];
        let mut field = [0; 3];
        for s in &self.sprouts {
            if s.in_field() {
                field[s.color as usize] += 1;
                if s.state == SproutState::Follow {
                    squad[s.color as usize] += 1;
                }
            }
        }
        (squad, field)
    }

    pub fn nearest_shard(&self) -> Option<Vec3> {
        let p = self.player.body.pos;
        self.items
            .iter()
            .filter(|i| i.kind == ItemKind::Shard)
            .map(|i| i.body.pos)
            .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
    }

    #[allow(dead_code)]
    pub fn in_world(p: Vec3) -> bool {
        p.x >= 0.0 && p.z >= 0.0 && p.x < SX as f32 && p.z < SZ as f32
    }
}

fn turn_towards(cur: f32, want: f32, max: f32) -> f32 {
    let mut d = (want - cur) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    }
    if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    cur + d.clamp(-max, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(g: &mut Game, input: &Input, secs: f32) {
        let n = (secs / 0.02) as usize;
        for _ in 0..n {
            g.update(input, 0.02);
        }
    }

    #[test]
    fn pluck_and_follow() {
        let mut g = Game::new(7);
        g.player.body.pos = g.nest + vec3(3.0, 0.0, 2.0);
        step(&mut g, &Input::default(), 0.5);
        let input = Input {
            pluck: true,
            ..Default::default()
        };
        step(&mut g, &input, 1.0);
        let (squad, _) = g.squad_counts();
        assert!(squad[C_RED as usize] >= 1, "squad {squad:?}");
    }

    #[test]
    fn sprouts_carry_blocks_home() {
        let mut g = Game::new(11);
        // Put a loose block a few steps from the nest and a red sprout next to it.
        let pos = g.nest + vec3(5.0, 0.5, 0.0);
        let id = g.new_id();
        g.items
            .push(Item::new(id, ItemKind::Block(Block::Stone), pos));
        let sid = g.new_id();
        g.sprouts.push(Sprout::new(
            sid,
            C_RED,
            pos + vec3(0.5, 0.0, 0.0),
            SproutState::Idle,
        ));
        let before = g.player.has(Block::Stone);
        g.player.body.pos = g.nest + vec3(-6.0, 0.0, -6.0);
        step(&mut g, &Input::default(), 20.0);
        assert_eq!(g.player.has(Block::Stone), before + 1);
    }

    #[test]
    fn sprouts_mine_thrown_target() {
        let mut g = Game::new(3);
        let target = g.world.nest + ivec3(3, -1, 0);
        assert!(g.world.get_v(target).is_solid());
        let sid = g.new_id();
        let mut s = Sprout::new(
            sid,
            C_YELLOW,
            target.as_vec3() + vec3(0.5, 1.0, 0.5),
            SproutState::Mine(target),
        );
        s.body.on_ground = true;
        g.sprouts.push(s);
        g.player.body.pos = g.nest + vec3(-6.0, 0.0, -6.0);
        step(&mut g, &Input::default(), 3.0);
        assert_eq!(g.world.get_v(target), Block::Air);
    }

    #[test]
    fn every_shard_can_be_carried_home() {
        for seed in [42u32, 2024] {
            let mut g = Game::new(seed);
            g.enemies.clear();
            g.player.body.pos = g.nest + vec3(-4.0, 0.0, -4.0);
            let shards: Vec<(u32, Vec3)> = g
                .items
                .iter()
                .filter(|i| i.kind == ItemKind::Shard)
                .map(|i| (i.id, i.body.pos))
                .collect();
            for (k, (_, pos)) in shards.iter().enumerate() {
                // Fire-proof reds for the lava moat (index 2), swimmers for the lake (index 3).
                let color = match k {
                    2 => C_RED,
                    3 => C_BLUE,
                    _ => C_YELLOW,
                };
                for n in 0..8 {
                    let id = g.new_id();
                    let a = n as f32 * 0.8;
                    let mut s = Sprout::new(
                        id,
                        color,
                        *pos + vec3(a.cos() * 0.6, 0.2, a.sin() * 0.6),
                        SproutState::Idle,
                    );
                    s.think = 0.0;
                    g.sprouts.push(s);
                }
            }
            for _ in 0..(600.0 / 0.05) as usize {
                g.update(&Input::default(), 0.05);
                g.time_of_day = 0.1; // stay in daylight
                if g.shards >= TOTAL_SHARDS {
                    break;
                }
            }
            let left: Vec<Vec3> = g
                .items
                .iter()
                .filter(|i| i.kind == ItemKind::Shard)
                .map(|i| i.body.pos)
                .collect();
            assert_eq!(
                g.shards, TOTAL_SHARDS,
                "seed {seed}: shards left at {left:?}, lost {}",
                g.stats.lost
            );
        }
    }

    #[test]
    fn crafting_consumes() {
        let mut g = Game::new(1);
        g.player.inv[Block::Log as usize] = 3;
        assert!(g.craft(0));
        assert_eq!(g.player.has(Block::Planks), 4);
        assert_eq!(g.player.has(Block::Log), 2);
    }
}
