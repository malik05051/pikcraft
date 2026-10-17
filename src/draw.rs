//! Drawing of creatures, items, effects and the HUD (terrain is drawn by `render`).

use crate::block::{Block, tile};
use crate::entities::*;
use crate::game::*;
use crate::render::{ATLAS_COLS, TILE};
use crate::world::SY;
use macroquad::prelude::*;

// ---------------------------------------------------------------- primitive

/// A shaded box with rotation, batched through macroquad's immediate renderer.
pub fn cube(
    gl: &mut QuadGl,
    c: Vec3,
    half: Vec3,
    rot: Mat3,
    rgb: [f32; 3],
    light: f32,
    alpha: f32,
) {
    const FACES: [([f32; 3], f32); 6] = [
        ([0.0, 1.0, 0.0], 1.0),
        ([0.0, -1.0, 0.0], 0.5),
        ([1.0, 0.0, 0.0], 0.82),
        ([-1.0, 0.0, 0.0], 0.82),
        ([0.0, 0.0, 1.0], 0.68),
        ([0.0, 0.0, -1.0], 0.68),
    ];
    let mut verts = Vec::with_capacity(24);
    let mut idx: Vec<u16> = Vec::with_capacity(36);
    for (n, shade) in FACES {
        let n = Vec3::from(n);
        let (u, v) = if n.y.abs() > 0.5 {
            (Vec3::X, Vec3::Z)
        } else if n.x.abs() > 0.5 {
            (Vec3::Y, Vec3::Z)
        } else {
            (Vec3::X, Vec3::Y)
        };
        // World-space shading so rotated boxes still look lit from above.
        let wn = rot * n;
        let s = if wn.y > 0.5 {
            1.0
        } else if wn.y < -0.5 {
            0.5
        } else {
            0.68 + wn.x.abs() * 0.14
        };
        let _ = shade;
        let k = s * light;
        let col = Color::new(rgb[0] * k, rgb[1] * k, rgb[2] * k, alpha);
        let base = verts.len() as u16;
        for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let local = (n + u * a + v * b) * half;
            let p = c + rot * local;
            verts.push(Vertex::new(p.x, p.y, p.z, 0.0, 0.0, col));
        }
        idx.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    gl.texture(None);
    gl.draw_mode(DrawMode::Triangles);
    gl.geometry(&verts, &idx);
}

fn yaw_rot(yaw: f32) -> Mat3 {
    // Local +X faces along the yaw direction (x = cos, z = sin).
    Mat3::from_rotation_y(-yaw)
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

// ---------------------------------------------------------------- creatures

pub fn draw_sprout(gl: &mut QuadGl, s: &Sprout, light: f32) {
    let rot = yaw_rot(s.yaw);
    let col = COLOR_RGB[s.color as usize];
    let base = s.body.pos;
    let at = |l: Vec3| base + rot * l;

    if s.state == SproutState::Buried {
        let sway = (s.anim * 2.0).sin() * 0.05;
        let stem = Mat3::from_rotation_z(sway);
        cube(
            gl,
            at(vec3(0.0, 0.12, 0.0)),
            vec3(0.018, 0.12, 0.018),
            rot * stem,
            [0.3, 0.6, 0.2],
            light,
            1.0,
        );
        draw_topper(gl, at(vec3(sway * 2.0, 0.26, 0.0)), rot, s, light);
        // A little colored tip peeking out of the soil hints at the colour.
        cube(
            gl,
            at(vec3(0.0, 0.01, 0.0)),
            vec3(0.08, 0.02, 0.08),
            rot,
            col,
            light * 0.8,
            1.0,
        );
        return;
    }

    let moving = s.body.vel.x.abs() + s.body.vel.z.abs() > 0.3;
    let walk = if moving { (s.anim * 14.0).sin() } else { 0.0 };
    let bob = walk.abs() * 0.03;
    let lift = if matches!(s.state, SproutState::Carry(_)) {
        0.0
    } else {
        bob
    };
    // Legs.
    for side in [-1.0f32, 1.0] {
        let swing = walk * 0.06 * side;
        cube(
            gl,
            at(vec3(swing, 0.06, side * 0.06)),
            vec3(0.04, 0.06, 0.035),
            rot,
            mix(col, [0.0; 3], 0.35),
            light,
            1.0,
        );
    }
    // Body and head.
    cube(
        gl,
        at(vec3(0.0, 0.23 + lift, 0.0)),
        vec3(0.1, 0.12, 0.09),
        rot,
        col,
        light,
        1.0,
    );
    let head = vec3(0.0, 0.47 + lift, 0.0);
    cube(gl, at(head), vec3(0.15, 0.14, 0.14), rot, col, light, 1.0);
    // Eyes.
    for side in [-1.0f32, 1.0] {
        cube(
            gl,
            at(head + vec3(0.145, 0.03, side * 0.06)),
            vec3(0.012, 0.035, 0.03),
            rot,
            [1.0, 1.0, 1.0],
            light.max(0.3),
            1.0,
        );
        cube(
            gl,
            at(head + vec3(0.155, 0.02, side * 0.06)),
            vec3(0.008, 0.02, 0.016),
            rot,
            [0.05, 0.05, 0.05],
            1.0,
            1.0,
        );
    }
    match s.color {
        C_RED => cube(
            gl,
            at(head + vec3(0.17, -0.04, 0.0)),
            vec3(0.035, 0.025, 0.025),
            rot,
            mix(col, [1.0; 3], 0.15),
            light,
            1.0,
        ),
        C_YELLOW => {
            for side in [-1.0f32, 1.0] {
                cube(
                    gl,
                    at(head + vec3(0.0, 0.05, side * 0.18)),
                    vec3(0.04, 0.07, 0.04),
                    rot,
                    col,
                    light,
                    1.0,
                );
            }
        }
        _ => cube(
            gl,
            at(head + vec3(0.15, -0.07, 0.0)),
            vec3(0.01, 0.012, 0.07),
            rot,
            [0.05, 0.1, 0.3],
            light,
            1.0,
        ),
    }
    // Arms raised while carrying or attacking.
    let arms_up = matches!(
        s.state,
        SproutState::Carry(_) | SproutState::Attack { .. } | SproutState::Mine(_)
    );
    for side in [-1.0f32, 1.0] {
        let (y, x) = if arms_up {
            (0.36 + (s.anim * 18.0).sin().abs() * 0.04, 0.06)
        } else {
            (0.22, 0.0)
        };
        cube(
            gl,
            at(vec3(x, y + lift, side * 0.13)),
            vec3(0.025, 0.06, 0.025),
            rot,
            col,
            light,
            1.0,
        );
    }
    // Stem and leaf.
    let sway = (s.anim * 3.0).sin() * 0.12 + s.body.vel.length() * -0.04;
    let tip = head + vec3(sway * 0.4, 0.3, 0.0);
    cube(
        gl,
        at(head + vec3(sway * 0.2, 0.22, 0.0)),
        vec3(0.015, 0.09, 0.015),
        rot * Mat3::from_rotation_z(-sway),
        [0.3, 0.6, 0.2],
        light,
        1.0,
    );
    draw_topper(gl, at(tip), rot, s, light);
}

fn draw_topper(gl: &mut QuadGl, p: Vec3, rot: Mat3, s: &Sprout, light: f32) {
    match s.maturity {
        0 => {
            let leaf = rot * Mat3::from_rotation_y(0.6) * Mat3::from_rotation_z(0.35);
            cube(
                gl,
                p + vec3(0.0, 0.02, 0.0),
                vec3(0.1, 0.012, 0.05),
                leaf,
                [0.32, 0.75, 0.25],
                light,
                1.0,
            );
        }
        1 => cube(
            gl,
            p,
            vec3(0.05, 0.06, 0.05),
            rot,
            [0.95, 0.85, 0.9],
            light,
            1.0,
        ),
        _ => {
            let petal = if s.color == C_BLUE {
                [0.85, 0.75, 1.0]
            } else {
                [1.0, 1.0, 1.0]
            };
            cube(gl, p, vec3(0.11, 0.025, 0.04), rot, petal, light, 1.0);
            cube(gl, p, vec3(0.04, 0.025, 0.11), rot, petal, light, 1.0);
            cube(
                gl,
                p + vec3(0.0, 0.02, 0.0),
                vec3(0.035, 0.02, 0.035),
                rot,
                [1.0, 0.85, 0.2],
                light,
                1.0,
            );
        }
    }
}

pub fn draw_chomper(gl: &mut QuadGl, e: &Enemy, light: f32, time: f32) {
    let k = e.size;
    let rot = yaw_rot(e.yaw);
    let base = e.body.pos;
    let at = |l: Vec3| base + rot * (l * k);
    let mut skin = [0.62, 0.22, 0.42];
    if e.nocturnal {
        skin = [0.35, 0.22, 0.55];
    }
    if e.hurt > 0.0 {
        skin = mix(skin, [1.0, 1.0, 1.0], 0.5);
    }
    let belly = mix(skin, [1.0, 0.9, 0.8], 0.45);
    let moving = e.body.vel.x.abs() + e.body.vel.z.abs() > 0.3;
    let walk = if moving { (time * 8.0).sin() } else { 0.0 };
    let shake = if e.state == EnemyState::Shake {
        (time * 40.0).sin() * 0.05
    } else {
        0.0
    };
    // Legs.
    for side in [-1.0f32, 1.0] {
        cube(
            gl,
            at(vec3(walk * 0.15 * side, 0.15, side * 0.32)),
            vec3(0.16, 0.15, 0.13),
            rot,
            mix(skin, [0.0; 3], 0.3),
            light,
            1.0,
        );
    }
    // Body.
    let body_c = vec3(-0.1 + shake, 0.72, 0.0);
    cube(
        gl,
        at(body_c),
        vec3(0.5, 0.42, 0.5) * k,
        rot,
        skin,
        light,
        1.0,
    );
    cube(
        gl,
        at(vec3(-0.1, 0.42, 0.0)),
        vec3(0.46, 0.08, 0.46) * k,
        rot,
        belly,
        light,
        1.0,
    );
    // Spots.
    for (dx, dz) in [(-0.3, 0.25), (0.05, -0.3), (-0.2, -0.1)] {
        cube(
            gl,
            at(vec3(dx, 1.15, dz)),
            vec3(0.09, 0.02, 0.09) * k,
            rot,
            belly,
            light,
            1.0,
        );
    }
    // Mouth: the upper jaw tilts back when biting.
    let open = e.mouth;
    let jaw_rot = rot * Mat3::from_rotation_z(open * 0.6);
    cube(
        gl,
        at(vec3(0.5, 0.82 + open * 0.15, 0.0)),
        vec3(0.22, 0.2, 0.45) * k,
        jaw_rot,
        skin,
        light,
        1.0,
    );
    cube(
        gl,
        at(vec3(0.5, 0.45, 0.0)),
        vec3(0.22, 0.09, 0.42) * k,
        rot,
        belly,
        light,
        1.0,
    );
    if open > 0.1 {
        cube(
            gl,
            at(vec3(0.55, 0.6, 0.0)),
            vec3(0.14, 0.12 * open, 0.36) * k,
            rot,
            [0.35, 0.05, 0.1],
            light,
            1.0,
        );
    }
    for i in 0..4 {
        let z = -0.3 + i as f32 * 0.2;
        cube(
            gl,
            at(vec3(0.68, 0.6 + open * 0.15, z)),
            vec3(0.03, 0.05, 0.03) * k,
            rot,
            [1.0, 1.0, 0.95],
            light,
            1.0,
        );
    }
    // Eye stalks.
    let asleep = e.state == EnemyState::Sleeping;
    for side in [-1.0f32, 1.0] {
        let stalk = vec3(0.3, 1.2, side * 0.22);
        cube(
            gl,
            at(stalk),
            vec3(0.05, 0.12, 0.05) * k,
            rot,
            skin,
            light,
            1.0,
        );
        let eye = stalk + vec3(0.02, 0.17, 0.0);
        if asleep {
            cube(
                gl,
                at(eye),
                vec3(0.1, 0.03, 0.1) * k,
                rot,
                mix(skin, [0.0; 3], 0.2),
                light,
                1.0,
            );
        } else {
            cube(
                gl,
                at(eye),
                vec3(0.1, 0.1, 0.1) * k,
                rot,
                [1.0, 1.0, 0.9],
                light.max(0.35),
                1.0,
            );
            let pupil = if e.nocturnal {
                [1.0, 0.2, 0.1]
            } else {
                [0.05, 0.05, 0.05]
            };
            cube(
                gl,
                at(eye + vec3(0.09, 0.0, 0.0)),
                vec3(0.02, 0.05, 0.05) * k,
                rot,
                pupil,
                1.0,
                1.0,
            );
        }
    }
}

fn draw_carcass(gl: &mut QuadGl, p: Vec3, size: f32, light: f32) {
    let rot = Mat3::from_rotation_x(std::f32::consts::PI);
    let c = [0.55, 0.42, 0.5];
    cube(
        gl,
        p + vec3(0.0, 0.4 * size, 0.0),
        vec3(0.5, 0.38, 0.45) * size,
        rot,
        c,
        light,
        1.0,
    );
    for side in [-1.0f32, 1.0] {
        cube(
            gl,
            p + vec3(0.0, 0.9 * size, side * 0.3 * size),
            vec3(0.12, 0.15, 0.1) * size,
            Mat3::IDENTITY,
            mix(c, [0.0; 3], 0.3),
            light,
            1.0,
        );
    }
}

fn draw_shard(gl: &mut QuadGl, p: Vec3, spin: f32) {
    let c = p + vec3(0.0, 0.55 + (spin * 2.0).sin() * 0.08, 0.0);
    let gold = [1.0, 0.85, 0.25];
    for k in 0..3 {
        let r = Mat3::from_rotation_y(spin + k as f32 * 0.52) * Mat3::from_rotation_z(0.785);
        cube(gl, c, vec3(0.28, 0.28, 0.06), r, gold, 1.15, 1.0);
    }
    cube(
        gl,
        c,
        vec3(0.12, 0.12, 0.12),
        Mat3::from_rotation_y(spin),
        [1.0, 1.0, 0.8],
        1.3,
        1.0,
    );
}

pub fn draw_nest(gl: &mut QuadGl, nest: Vec3, pulse: f32, time: f32, light: f32) {
    let s = 1.0 + pulse * 0.15 * (time * 20.0).sin().abs();
    let legs = [0.0f32, 2.094, 4.189];
    for a in legs {
        let d = vec3(a.cos(), 0.0, a.sin());
        let r = Mat3::from_rotation_y(-a) * Mat3::from_rotation_z(0.45);
        cube(
            gl,
            nest + d * 0.9 + vec3(0.0, 0.8, 0.0),
            vec3(0.1, 0.85, 0.1),
            r,
            [0.35, 0.25, 0.15],
            light,
            1.0,
        );
    }
    let bands = [[0.92, 0.18, 0.14], [1.0, 0.86, 0.15], [0.2, 0.42, 0.98]];
    let rot = Mat3::from_rotation_y(time * 0.2);
    for (i, b) in bands.iter().enumerate() {
        let y = 1.75 + i as f32 * 0.42;
        let w = [0.95, 1.05, 0.85][i] * s;
        cube(
            gl,
            nest + vec3(0.0, y, 0.0),
            vec3(w, 0.22, w),
            rot,
            *b,
            light.max(0.5),
            1.0,
        );
    }
    cube(
        gl,
        nest + vec3(0.0, 3.05, 0.0),
        vec3(0.55, 0.15, 0.55) * s,
        rot,
        [0.95, 0.92, 0.8],
        light.max(0.5),
        1.0,
    );
    cube(
        gl,
        nest + vec3(0.0, 3.45, 0.0),
        vec3(0.04, 0.25, 0.04),
        rot,
        [0.3, 0.6, 0.2],
        light,
        1.0,
    );
    cube(
        gl,
        nest + vec3(0.12, 3.72, 0.0),
        vec3(0.18, 0.02, 0.08),
        rot * Mat3::from_rotation_z(0.3),
        [0.32, 0.75, 0.25],
        light,
        1.0,
    );
}

fn beam(gl: &mut QuadGl, base: Vec3, rgb: [f32; 3], alpha: f32) {
    let h = (SY as f32 + 30.0 - base.y).max(1.0);
    cube(
        gl,
        base + vec3(0.0, h * 0.5, 0.0),
        vec3(0.12, h * 0.5, 0.12),
        Mat3::IDENTITY,
        rgb,
        1.0,
        alpha,
    );
}

// ---------------------------------------------------------------- scene

pub fn draw_entities(g: &Game, time: f32, throw_preview: Option<Vec3>) {
    let gl = unsafe { get_internal_gl() }.quad_gl;

    // Sun and moon.
    let ang = g.time_of_day * std::f32::consts::TAU;
    let dir = vec3(ang.cos(), ang.sin(), 0.35).normalize();
    let eye = g.player.eye();
    cube(
        gl,
        eye + dir * 300.0,
        vec3(14.0, 14.0, 14.0),
        Mat3::IDENTITY,
        [1.0, 0.95, 0.7],
        1.0,
        1.0,
    );
    cube(
        gl,
        eye - dir * 300.0,
        vec3(9.0, 9.0, 9.0),
        Mat3::IDENTITY,
        [0.85, 0.88, 0.95],
        1.0,
        1.0,
    );

    let nl = g.light_mul(g.nest + vec3(0.0, 1.0, 0.0));
    draw_nest(gl, g.nest, g.nest_pulse, time, nl);

    for s in &g.sprouts {
        if s.body.pos.distance(eye) > 90.0 {
            continue;
        }
        draw_sprout(gl, s, g.light_mul(s.body.pos));
    }
    for e in &g.enemies {
        if e.body.pos.distance(eye) > 110.0 {
            continue;
        }
        draw_chomper(gl, e, g.light_mul(e.body.pos), time + e.id as f32);
    }
    for it in &g.items {
        let l = g.light_mul(it.body.pos);
        match it.kind {
            ItemKind::Block(b) => {
                let bob = if it.carriers.is_empty() {
                    (it.spin * 3.0).sin() * 0.05 + 0.2
                } else {
                    0.45
                };
                let c = it.body.pos + vec3(0.0, bob, 0.0);
                cube(
                    gl,
                    c,
                    vec3(0.15, 0.15, 0.15),
                    Mat3::from_rotation_y(it.spin),
                    b.color(),
                    l * 1.1,
                    1.0,
                );
            }
            ItemKind::Carcass { size, .. } => {
                let lift = if it.carriers.len() >= it.weight() {
                    0.3
                } else {
                    0.0
                };
                draw_carcass(gl, it.body.pos + vec3(0.0, lift, 0.0), size * 0.8, l);
            }
            ItemKind::Shard => draw_shard(gl, it.body.pos, it.spin),
        }
    }
    for p in &g.particles {
        let a = (p.life * 2.0).min(1.0);
        cube(
            gl,
            p.pos,
            Vec3::splat(p.size),
            Mat3::IDENTITY,
            p.color,
            1.0,
            a,
        );
    }

    // Whistle ring.
    if g.player.whistle > 0.0 {
        let c = g.player.whistle_center + vec3(0.0, 0.15, 0.0);
        let r = g.player.whistle;
        let n = 40;
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU + time * 2.0;
            let p = c + vec3(
                a.cos() * r,
                (time * 8.0 + i as f32).sin() * 0.05,
                a.sin() * r,
            );
            cube(
                gl,
                p,
                vec3(0.08, 0.03, 0.08),
                Mat3::from_rotation_y(-a),
                [1.0, 0.95, 0.4],
                1.2,
                0.9,
            );
        }
    }
    if let Some(p) = throw_preview {
        let c = HOTBAR_COLOR(g.player.throw_color);
        for i in 0..12 {
            let a = i as f32 / 12.0 * std::f32::consts::TAU + time * 3.0;
            cube(
                gl,
                p + vec3(a.cos() * 0.4, 0.06, a.sin() * 0.4),
                vec3(0.05, 0.02, 0.05),
                Mat3::IDENTITY,
                c,
                1.2,
                0.9,
            );
        }
    }
}

/// Translucent beacons over the nest and every remaining shard; drawn after all opaque geometry.
pub fn draw_beams(g: &Game) {
    let gl = unsafe { get_internal_gl() }.quad_gl;
    let ba = 0.18 + (1.0 - g.daylight()) * 0.1;
    beam(gl, g.nest + vec3(0.0, 3.8, 0.0), [0.6, 1.0, 0.6], ba);
    for it in &g.items {
        if it.kind == ItemKind::Shard {
            beam(gl, it.body.pos + vec3(0.0, 1.0, 0.0), [1.0, 0.85, 0.3], ba);
        }
    }
}

#[allow(non_snake_case)]
fn HOTBAR_COLOR(c: u8) -> [f32; 3] {
    COLOR_RGB[c as usize]
}

/// Block highlight and crack overlay.
pub fn draw_block_cursor(g: &Game, pos: IVec3) {
    let c = pos.as_vec3() + Vec3::splat(0.5);
    draw_cube_wires(c, Vec3::splat(1.005), Color::new(0.0, 0.0, 0.0, 0.6));
    let prog = g.dig_progress(pos);
    if prog > 0.0 {
        let gl = unsafe { get_internal_gl() }.quad_gl;
        cube(
            gl,
            c,
            Vec3::splat(0.503),
            Mat3::IDENTITY,
            [0.0, 0.0, 0.0],
            1.0,
            prog * 0.55,
        );
    }
}

/// First-person hand, holding a block or the next sprout to throw.
pub fn draw_hand(g: &Game) {
    let gl = unsafe { get_internal_gl() }.quad_gl;
    let p = &g.player;
    let fwd = p.forward();
    let right = p.right();
    let up = right.cross(fwd).normalize();
    let swing = (p.swing * std::f32::consts::PI).sin();
    let rot = Mat3::from_cols(fwd, up, right);
    let base = p.eye() + fwd * (0.6 - swing * 0.12) + right * 0.36 - up * (0.32 - swing * 0.08);
    let light = g.light_mul(p.eye());
    cube(
        gl,
        base - fwd * 0.1 - up * 0.04,
        vec3(0.13, 0.035, 0.035),
        rot,
        [0.86, 0.66, 0.5],
        light,
        1.0,
    );
    match HOTBAR[p.slot] {
        Some(b) if p.has(b) > 0 => {
            cube(
                gl,
                base + up * 0.03,
                Vec3::splat(0.05),
                rot,
                b.color(),
                light,
                1.0,
            );
        }
        None => {
            let (squad, _) = g.squad_counts();
            if squad[p.throw_color as usize] > 0 {
                let c = COLOR_RGB[p.throw_color as usize];
                cube(gl, base + up * 0.04, Vec3::splat(0.035), rot, c, light, 1.0);
                cube(
                    gl,
                    base + up * 0.1,
                    vec3(0.005, 0.03, 0.005),
                    rot,
                    [0.3, 0.6, 0.2],
                    light,
                    1.0,
                );
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------- HUD

pub struct Hud {
    pub atlas: Texture2D,
}

impl Hud {
    pub fn new() -> Hud {
        let px = crate::render::atlas_pixels();
        let n = crate::render::ATLAS_PX as u16;
        let atlas = Texture2D::from_rgba8(n, n, &px);
        atlas.set_filter(FilterMode::Nearest);
        Hud { atlas }
    }

    pub fn icon(&self, b: Block, x: f32, y: f32, size: f32, alpha: f32) {
        let t = b.tile(2) as usize;
        let t = if b == Block::Grass {
            tile::GRASS_SIDE as usize
        } else {
            t
        };
        let src = Rect::new(
            ((t % ATLAS_COLS) * TILE) as f32,
            ((t / ATLAS_COLS) * TILE) as f32,
            TILE as f32,
            TILE as f32,
        );
        draw_texture_ex(
            &self.atlas,
            x,
            y,
            Color::new(1.0, 1.0, 1.0, alpha),
            DrawTextureParams {
                dest_size: Some(vec2(size, size)),
                source: Some(src),
                ..Default::default()
            },
        );
    }
}

pub fn text_shadow(s: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text(
        s,
        x + 2.0,
        y + 2.0,
        size,
        Color::new(0.0, 0.0, 0.0, 0.6 * color.a),
    );
    draw_text(s, x, y, size, color);
}

pub fn text_center(s: &str, cx: f32, y: f32, size: f32, color: Color) {
    let m = measure_text(s, None, size as u16, 1.0);
    text_shadow(s, cx - m.width / 2.0, y, size, color);
}

fn to_color(c: [f32; 3]) -> Color {
    Color::new(c[0], c[1], c[2], 1.0)
}

pub fn project(mvp: &Mat4, p: Vec3) -> Option<Vec2> {
    let c = *mvp * p.extend(1.0);
    if c.w <= 0.05 {
        return None;
    }
    let ndc = c.truncate() / c.w;
    if ndc.x.abs() > 1.2 || ndc.y.abs() > 1.2 {
        return None;
    }
    Some(vec2(
        (ndc.x + 1.0) * 0.5 * screen_width(),
        (1.0 - ndc.y) * 0.5 * screen_height(),
    ))
}

pub fn draw_hud(g: &Game, hud: &Hud, mvp: &Mat4, target: &Target) {
    let (w, h) = (screen_width(), screen_height());
    let p = &g.player;

    if p.body.head_in_water {
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.1, 0.25, 0.6, 0.35));
    }
    if p.body.in_lava {
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.9, 0.3, 0.0, 0.45));
    }
    if p.hurt > 0.0 {
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.8, 0.0, 0.0, p.hurt * 0.5));
    }

    // Labels over carryable items.
    for it in &g.items {
        if it.kind == ItemKind::Block(Block::Air) {
            continue;
        }
        let show = !matches!(it.kind, ItemKind::Block(_)) || !it.carriers.is_empty();
        if !show || it.body.pos.distance(p.body.pos) > 24.0 {
            continue;
        }
        if let Some(s) = project(mvp, it.body.pos + vec3(0.0, 1.5, 0.0)) {
            let txt = format!("{}/{}", it.carriers.len(), it.weight());
            let col = if it.carriers.len() >= it.weight() {
                if it.no_path {
                    Color::new(1.0, 0.5, 0.3, 1.0)
                } else {
                    Color::new(0.6, 1.0, 0.6, 1.0)
                }
            } else {
                WHITE
            };
            text_center(&txt, s.x, s.y, 26.0, col);
            if it.no_path && it.carriers.len() >= it.weight() {
                text_center("can't get home!", s.x, s.y + 18.0, 18.0, col);
            }
        }
    }

    // Crosshair.
    let (cx, cy) = (w / 2.0, h / 2.0);
    draw_rectangle(
        cx - 9.0,
        cy - 1.0,
        18.0,
        2.0,
        Color::new(1.0, 1.0, 1.0, 0.85),
    );
    draw_rectangle(
        cx - 1.0,
        cy - 9.0,
        2.0,
        18.0,
        Color::new(1.0, 1.0, 1.0, 0.85),
    );

    // What we're looking at.
    let label = match *target {
        Target::Block { pos, dist, .. } if dist < 5.0 => {
            let b = g.world.get_v(pos);
            let prog = g.dig_progress(pos);
            if prog > 0.0 {
                draw_rectangle(
                    cx - 40.0,
                    cy + 22.0,
                    80.0,
                    6.0,
                    Color::new(0.0, 0.0, 0.0, 0.5),
                );
                draw_rectangle(
                    cx - 40.0,
                    cy + 22.0,
                    80.0 * prog,
                    6.0,
                    Color::new(1.0, 1.0, 1.0, 0.9),
                );
            }
            Some(b.name().to_string())
        }
        Target::Enemy { id, .. } => g.enemies.iter().find(|e| e.id == id).map(|e| {
            let state = if e.state == EnemyState::Sleeping {
                " (asleep)"
            } else {
                ""
            };
            format!("Chomper {:.0}/{:.0}{}", e.hp.max(0.0), e.max_hp, state)
        }),
        Target::Item { id, .. } => g
            .items
            .iter()
            .find(|i| i.id == id)
            .map(|i| format!("{} ({}/{})", i.label(), i.carriers.len(), i.weight())),
        Target::Sprout { id, .. } => g.sprouts.iter().find(|s| s.id == id).map(|s| {
            format!(
                "{} sprout - press [E] to pluck",
                COLOR_NAMES[s.color as usize]
            )
        }),
        _ => None,
    };
    if let Some(l) = label {
        text_center(&l, cx, cy + 48.0, 22.0, Color::new(1.0, 1.0, 1.0, 0.9));
    }

    // Hotbar.
    let slot = 52.0;
    let bx = cx - slot * 9.0 / 2.0;
    let by = h - slot - 14.0;
    let (squad, field) = g.squad_counts();
    for (i, b) in HOTBAR.iter().enumerate() {
        let x = bx + i as f32 * slot;
        draw_rectangle(
            x,
            by,
            slot - 4.0,
            slot - 4.0,
            Color::new(0.0, 0.0, 0.0, 0.45),
        );
        if i == p.slot {
            draw_rectangle_lines(
                x - 2.0,
                by - 2.0,
                slot,
                slot,
                4.0,
                Color::new(1.0, 1.0, 1.0, 0.95),
            );
        }
        match b {
            None => {
                let c = COLOR_RGB[p.throw_color as usize];
                draw_rectangle(x + 12.0, by + 16.0, 24.0, 22.0, to_color(c));
                draw_rectangle(x + 16.0, by + 22.0, 4.0, 5.0, BLACK);
                draw_rectangle(x + 28.0, by + 22.0, 4.0, 5.0, BLACK);
                draw_rectangle(
                    x + 23.0,
                    by + 6.0,
                    2.0,
                    10.0,
                    Color::new(0.3, 0.6, 0.2, 1.0),
                );
                draw_rectangle(
                    x + 23.0,
                    by + 4.0,
                    10.0,
                    4.0,
                    Color::new(0.35, 0.75, 0.25, 1.0),
                );
                let n: u32 = squad.iter().sum();
                text_shadow(&n.to_string(), x + 4.0, by + slot - 8.0, 20.0, WHITE);
            }
            Some(b) => {
                let n = p.has(*b);
                hud.icon(
                    *b,
                    x + 8.0,
                    by + 8.0,
                    slot - 20.0,
                    if n > 0 { 1.0 } else { 0.25 },
                );
                if n > 0 {
                    text_shadow(&n.to_string(), x + 4.0, by + slot - 8.0, 20.0, WHITE);
                }
            }
        }
        text_shadow(
            &(i + 1).to_string(),
            x + 3.0,
            by + 13.0,
            14.0,
            Color::new(1.0, 1.0, 1.0, 0.6),
        );
    }
    let mode = match HOTBAR[p.slot] {
        None => {
            "SQUAD: click/[F] throw  right-click/[R] whistle  [X] dismiss  [Tab] colour".to_string()
        }
        Some(b) => format!(
            "{}: click mine/attack  right-click place   ({})",
            b.name(),
            TOOL_NAMES[p.tool as usize]
        ),
    };
    text_center(&mode, cx, by - 10.0, 18.0, Color::new(1.0, 1.0, 1.0, 0.8));

    // Health.
    for i in 0..10 {
        let x = 16.0 + i as f32 * 22.0;
        let fill = ((p.hp - i as f32 * 2.0) / 2.0).clamp(0.0, 1.0);
        draw_rectangle(x, 16.0, 18.0, 16.0, Color::new(0.15, 0.0, 0.0, 0.6));
        draw_rectangle(x, 16.0, 18.0 * fill, 16.0, Color::new(0.9, 0.1, 0.15, 1.0));
    }
    if p.air < 10.0 {
        for i in 0..10 {
            if p.air > i as f32 {
                draw_circle(
                    25.0 + i as f32 * 22.0,
                    46.0,
                    7.0,
                    Color::new(0.6, 0.85, 1.0, 0.9),
                );
            }
        }
    }

    // Squad panel.
    let py = h - 120.0;
    draw_rectangle(
        10.0,
        py - 26.0,
        200.0,
        104.0,
        Color::new(0.0, 0.0, 0.0, 0.4),
    );
    text_shadow("Squad / Field", 18.0, py - 8.0, 20.0, WHITE);
    for c in 0..3 {
        let y = py + 6.0 + c as f32 * 22.0;
        draw_rectangle(18.0, y, 14.0, 14.0, to_color(COLOR_RGB[c]));
        if c as u8 == p.throw_color {
            draw_rectangle_lines(15.0, y - 3.0, 190.0, 20.0, 2.0, WHITE);
        }
        text_shadow(
            &format!("{:<7}{:>3} / {:<3}", COLOR_NAMES[c], squad[c], field[c]),
            40.0,
            y + 13.0,
            20.0,
            WHITE,
        );
    }

    // Top right: day, clock and shards.
    let night = g.is_night();
    let clock = if night { "Night" } else { "Day" };
    let tr = format!("{} {}   Shards {}/{}", clock, g.day, g.shards, TOTAL_SHARDS);
    let m = measure_text(&tr, None, 26, 1.0);
    draw_rectangle(
        w - m.width - 30.0,
        10.0,
        m.width + 20.0,
        34.0,
        Color::new(0.0, 0.0, 0.0, 0.4),
    );
    text_shadow(
        &tr,
        w - m.width - 20.0,
        35.0,
        26.0,
        if night {
            Color::new(0.7, 0.75, 1.0, 1.0)
        } else {
            WHITE
        },
    );
    // Day progress bar.
    let frac = g.time_of_day;
    draw_rectangle(
        w - m.width - 30.0,
        46.0,
        m.width + 20.0,
        5.0,
        Color::new(0.0, 0.0, 0.0, 0.4),
    );
    draw_rectangle(
        w - m.width - 30.0 + (m.width + 20.0) * frac - 3.0,
        44.0,
        6.0,
        9.0,
        YELLOW,
    );

    // Shard compass.
    if let Some(sp) = g.nearest_shard() {
        let d = sp - p.body.pos;
        let ang = d.z.atan2(d.x) - p.yaw;
        let dist = vec2(d.x, d.z).length();
        let (ax, ay) = (cx, 40.0);
        let tip = vec2(ax + ang.sin() * 16.0, ay - ang.cos() * 16.0);
        let l = vec2(ax + (ang + 2.5).sin() * 10.0, ay - (ang + 2.5).cos() * 10.0);
        let r = vec2(ax + (ang - 2.5).sin() * 10.0, ay - (ang - 2.5).cos() * 10.0);
        draw_circle(ax, ay, 20.0, Color::new(0.0, 0.0, 0.0, 0.4));
        draw_triangle(tip, l, r, Color::new(1.0, 0.85, 0.25, 1.0));
        text_center(
            &format!("Star Shard {:.0}m", dist),
            ax,
            ay + 38.0,
            18.0,
            Color::new(1.0, 0.9, 0.5, 0.9),
        );
    }

    // Toasts.
    let mut y = 90.0;
    for t in &g.toasts {
        let a = t.time.min(1.0);
        text_shadow(&t.text, 18.0, y, 22.0, Color::new(1.0, 1.0, 0.85, a));
        y += 26.0;
    }
}

pub fn draw_panel(x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(x, y, w, h, Color::new(0.05, 0.07, 0.05, 0.86));
    draw_rectangle_lines(x, y, w, h, 3.0, Color::new(0.45, 0.8, 0.35, 1.0));
}

pub fn draw_crafting(g: &Game, hud: &Hud, hover: Option<usize>) -> Vec<Rect> {
    let (w, h) = (screen_width(), screen_height());
    let (pw, ph) = (640.0, 560.0f32.min(h - 40.0));
    let (px, py) = ((w - pw) / 2.0, (h - ph) / 2.0);
    draw_panel(px, py, pw, ph);
    text_center(
        "Crafting  (click a recipe or press its number, [C] to close)",
        w / 2.0,
        py + 32.0,
        22.0,
        WHITE,
    );
    let mut rects = Vec::new();
    for (i, r) in RECIPES.iter().enumerate() {
        let ry = py + 50.0 + i as f32 * 44.0;
        let rect = Rect::new(px + 16.0, ry, pw - 32.0, 40.0);
        let ok = g.can_craft(r);
        let bg = if Some(i) == hover && ok {
            Color::new(0.3, 0.5, 0.25, 0.9)
        } else {
            Color::new(0.0, 0.0, 0.0, 0.35)
        };
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
        let col = if ok {
            WHITE
        } else {
            Color::new(0.6, 0.6, 0.6, 1.0)
        };
        text_shadow(
            &format!("{}. {}", i + 1, r.name),
            rect.x + 10.0,
            ry + 27.0,
            22.0,
            col,
        );
        let mut x = rect.x + 260.0;
        for (b, n) in r.needs {
            hud.icon(*b, x, ry + 6.0, 28.0, 1.0);
            let have = g.player.has(*b);
            let c = if have >= *n {
                Color::new(0.7, 1.0, 0.7, 1.0)
            } else {
                Color::new(1.0, 0.55, 0.5, 1.0)
            };
            text_shadow(
                &format!("{}/{}", have.min(999), n),
                x + 32.0,
                ry + 27.0,
                20.0,
                c,
            );
            x += 100.0;
        }
        rects.push(rect);
    }
    // Inventory summary.
    let iy = py + 50.0 + RECIPES.len() as f32 * 44.0 + 14.0;
    text_shadow("Materials:", px + 16.0, iy + 4.0, 20.0, WHITE);
    let mut x = px + 16.0;
    let mut y = iy + 14.0;
    for b in crate::block::ALL_BLOCKS {
        let n = g.player.has(b);
        if n == 0 {
            continue;
        }
        hud.icon(b, x, y, 26.0, 1.0);
        text_shadow(&n.to_string(), x + 30.0, y + 20.0, 18.0, WHITE);
        x += 76.0;
        if x > px + pw - 80.0 {
            x = px + 16.0;
            y += 32.0;
        }
    }
    text_shadow(
        &format!("Tool: {}", TOOL_NAMES[g.player.tool as usize]),
        px + 16.0,
        py + ph - 14.0,
        20.0,
        Color::new(1.0, 0.9, 0.6, 1.0),
    );
    rects
}

pub const HELP: &[&str] = &[
    "PikCraft - a block world with a squad of sprouts",
    "",
    "GOAL: find the 5 Star Shards (follow the compass and golden beams)",
    "and have your sprouts carry them to the Nest (green beam).",
    "",
    "WASD move   Space jump/swim   Shift sprint   Mouse look",
    "1 squad slot   2-9 blocks   Mouse wheel cycles slots",
    "Squad slot: Left-click throw sprout   Right-click whistle",
    "Block slots: Left-click mine/attack   Right-click place",
    "F throw   R whistle (hold)   E pluck sprouts   X dismiss squad",
    "Tab change thrown colour   C crafting   F1 help   Esc pause",
    "",
    "Throw sprouts at blocks to dig, at chompers to attack, at",
    "items to carry them home. Loot carried home goes to your bag.",
    "RED resist fire and hit hard.  YELLOW fly high and dig fast.",
    "BLUE can swim - others drown in deep water.",
    "Chompers sleep by day and hunt at night. Defeated chompers",
    "carried to the Nest grow new sprout seeds.",
];

pub fn draw_help() {
    let (w, h) = (screen_width(), screen_height());
    let (pw, ph) = (
        720.0f32.min(w - 20.0),
        (HELP.len() as f32 * 26.0 + 60.0).min(h - 20.0),
    );
    let (px, py) = ((w - pw) / 2.0, (h - ph) / 2.0);
    draw_panel(px, py, pw, ph);
    for (i, l) in HELP.iter().enumerate() {
        let size = if i == 0 { 30.0 } else { 22.0 };
        let col = if i == 0 {
            Color::new(0.6, 1.0, 0.5, 1.0)
        } else {
            WHITE
        };
        text_shadow(l, px + 24.0, py + 40.0 + i as f32 * 26.0, size, col);
    }
    text_center(
        "Press F1 to close",
        w / 2.0,
        py + ph - 12.0,
        20.0,
        Color::new(1.0, 1.0, 1.0, 0.7),
    );
}
