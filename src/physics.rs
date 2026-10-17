//! Axis-aligned box physics against the voxel grid, shared by the player and creatures.

use crate::block::Block;
use crate::world::World;
use macroquad::math::{Vec3, vec3};

pub const GRAVITY: f32 = 26.0;

#[derive(Clone, Debug)]
pub struct Body {
    /// Centre of the feet.
    pub pos: Vec3,
    pub vel: Vec3,
    pub hw: f32,
    pub h: f32,
    pub on_ground: bool,
    pub hit_wall: bool,
    pub in_water: bool,
    pub head_in_water: bool,
    pub in_lava: bool,
    /// Downward speed at the moment of the last landing (for fall damage).
    pub landed_speed: f32,
}

impl Body {
    pub fn new(pos: Vec3, hw: f32, h: f32) -> Body {
        Body {
            pos,
            vel: Vec3::ZERO,
            hw,
            h,
            on_ground: false,
            hit_wall: false,
            in_water: false,
            head_in_water: false,
            in_lava: false,
            landed_speed: 0.0,
        }
    }

    pub fn center(&self) -> Vec3 {
        self.pos + vec3(0.0, self.h * 0.5, 0.0)
    }

    pub fn collides_at(&self, w: &World, p: Vec3) -> bool {
        let e = 0.001;
        let (x0, x1) = (
            (p.x - self.hw + e).floor() as i32,
            (p.x + self.hw - e).floor() as i32,
        );
        let (y0, y1) = ((p.y + e).floor() as i32, (p.y + self.h - e).floor() as i32);
        let (z0, z1) = (
            (p.z - self.hw + e).floor() as i32,
            (p.z + self.hw - e).floor() as i32,
        );
        for x in x0..=x1 {
            for y in y0..=y1 {
                for z in z0..=z1 {
                    if w.solid(x, y, z) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn sample(&self, w: &World, y_off: f32) -> Block {
        let p = self.pos + vec3(0.0, y_off, 0.0);
        w.get(p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32)
    }

    pub fn update_fluids(&mut self, w: &World) {
        let feet = self.sample(w, 0.1);
        let mid = self.sample(w, self.h * 0.5);
        let head = self.sample(w, self.h * 0.85);
        self.in_water = feet == Block::Water || mid == Block::Water;
        self.head_in_water = head == Block::Water;
        self.in_lava = feet == Block::Lava || mid == Block::Lava;
    }

    /// Integrate velocity with gravity and resolve collisions axis by axis.
    pub fn step(&mut self, w: &World, dt: f32, gravity: f32) {
        self.update_fluids(w);
        let g = if self.in_water || self.in_lava {
            gravity * 0.35
        } else {
            gravity
        };
        self.vel.y -= g * dt;
        if self.in_water || self.in_lava {
            let drag = (1.0 - 2.5 * dt).max(0.0);
            self.vel.y *= drag;
            self.vel.y = self.vel.y.max(-4.0);
        }
        self.vel.y = self.vel.y.max(-40.0);

        let was_on_ground = self.on_ground;
        let fall_speed = -self.vel.y;
        self.on_ground = false;
        self.hit_wall = false;
        let total = self.vel * dt;
        let steps = (total.abs().max_element() / 0.3).ceil().max(1.0) as i32;
        let d = total / steps as f32;
        for _ in 0..steps {
            for axis in [1usize, 0, 2] {
                let delta = d[axis];
                if delta == 0.0 {
                    continue;
                }
                let mut np = self.pos;
                np[axis] += delta;
                if !self.collides_at(w, np) {
                    self.pos = np;
                    continue;
                }
                // Snap flush against the blocking cell.
                let e = 0.0005;
                match axis {
                    1 => {
                        if delta < 0.0 {
                            np.y = np.y.floor() + 1.0;
                            if !self.collides_at(w, np) {
                                self.pos = np;
                            }
                            self.on_ground = true;
                            if !was_on_ground {
                                self.landed_speed = fall_speed;
                            }
                        } else {
                            np.y = (np.y + self.h).floor() - self.h - e;
                            if !self.collides_at(w, np) {
                                self.pos = np;
                            }
                        }
                        self.vel.y = 0.0;
                    }
                    _ => {
                        if delta > 0.0 {
                            np[axis] = (np[axis] + self.hw).floor() - self.hw - e;
                        } else {
                            np[axis] = (np[axis] - self.hw).floor() + 1.0 + self.hw + e;
                        }
                        if !self.collides_at(w, np) {
                            self.pos = np;
                        }
                        self.vel[axis] = 0.0;
                        self.hit_wall = true;
                    }
                }
            }
        }
        // Grounded check for bodies resting exactly on a surface.
        if !self.on_ground
            && self.vel.y <= 0.0
            && self.collides_at(w, self.pos - vec3(0.0, 0.01, 0.0))
        {
            self.on_ground = true;
        }
        // Escape if something got built inside us.
        if self.collides_at(w, self.pos) {
            for k in 1..4 {
                let up = self.pos + vec3(0.0, k as f32, 0.0);
                if !self.collides_at(w, up) {
                    self.pos = up;
                    break;
                }
            }
        }
    }

    /// Steer horizontally towards `target` at `speed`, hopping up single-block steps.
    pub fn steer(
        &mut self,
        w: &World,
        target: Vec3,
        speed: f32,
        accel: f32,
        dt: f32,
        jump: f32,
    ) -> f32 {
        let mut to = target - self.pos;
        to.y = 0.0;
        let dist = to.length();
        let want = if dist > 0.05 {
            to / dist * speed.min(dist * 6.0)
        } else {
            Vec3::ZERO
        };
        let k = (accel * dt).min(1.0);
        self.vel.x += (want.x - self.vel.x) * k;
        self.vel.z += (want.z - self.vel.z) * k;
        if jump > 0.0 && self.hit_wall && (self.on_ground || self.in_water) && dist > 0.3 {
            self.vel.y = jump;
        }
        if self.in_water && target.y > self.pos.y + 0.3 {
            self.vel.y = self.vel.y.max(2.0);
        }
        let _ = w;
        dist
    }
}
