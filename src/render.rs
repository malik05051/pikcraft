//! Procedural texture atlas, chunk meshing and the raw-miniquad terrain renderer.

use crate::block::{Block, tile};
use crate::noise::hash3;
use crate::world::{CHUNK, CX, CZ, SY, World};
use macroquad::math::{IVec3, Mat4, Vec3, Vec4, ivec3};
use macroquad::miniquad::*;

pub const TILE: usize = 16;
pub const ATLAS_COLS: usize = 8;
pub const ATLAS_PX: usize = TILE * ATLAS_COLS;

// ---------------------------------------------------------------- texture atlas

fn px_noise(seed: u32, t: u8, x: usize, y: usize) -> f32 {
    hash3(seed ^ (t as u32 * 7717), x as i32, y as i32, 3) as f32 / u32::MAX as f32
}

fn tile_pixel(t: u8, x: usize, y: usize) -> [u8; 4] {
    let n = px_noise(1, t, x, y);
    let n2 = px_noise(2, t, x / 2, y / 2);
    let shade = |c: [f32; 3], k: f32| -> [u8; 4] {
        [
            (c[0] * k * 255.0).clamp(0.0, 255.0) as u8,
            (c[1] * k * 255.0).clamp(0.0, 255.0) as u8,
            (c[2] * k * 255.0).clamp(0.0, 255.0) as u8,
            255,
        ]
    };
    let grain = 0.86 + n * 0.18;
    let stone = |n: f32, n2: f32| shade([0.52, 0.52, 0.54], 0.82 + n * 0.12 + n2 * 0.12);
    let ore = |c: [f32; 3]| {
        let blob = px_noise(9, t, x / 3, y / 3) > 0.68 && n > 0.2;
        if blob {
            shade(c, 0.85 + n * 0.25)
        } else {
            stone(n, n2)
        }
    };
    match t {
        tile::GRASS_TOP => shade([0.36, 0.66, 0.26], 0.8 + n * 0.3),
        tile::GRASS_SIDE => {
            let edge = 3 + (px_noise(5, t, x, 0) * 3.0) as usize;
            if y < edge {
                shade([0.36, 0.66, 0.26], 0.8 + n * 0.3)
            } else {
                shade([0.53, 0.37, 0.24], grain)
            }
        }
        tile::DIRT => shade([0.53, 0.37, 0.24], grain),
        tile::STONE => stone(n, n2),
        tile::SAND => shade([0.86, 0.80, 0.56], 0.92 + n * 0.1),
        tile::GRAVEL => shade([0.50, 0.47, 0.45], 0.7 + n2 * 0.4),
        tile::WATER => {
            let mut c = shade([0.22, 0.42, 0.86], 0.85 + n2 * 0.2);
            c[3] = 170;
            c
        }
        tile::LOG_SIDE => {
            let stripe = if (x + (n2 * 2.0) as usize) % 4 == 0 {
                0.75
            } else {
                1.0
            };
            shade([0.45, 0.32, 0.18], stripe * (0.9 + n * 0.12))
        }
        tile::LOG_TOP => {
            let (dx, dy) = (x as f32 - 7.5, y as f32 - 7.5);
            let r = (dx * dx + dy * dy).sqrt();
            if r > 7.0 {
                shade([0.45, 0.32, 0.18], 0.9)
            } else {
                let ring = if (r as i32) % 3 == 0 { 0.85 } else { 1.0 };
                shade([0.74, 0.58, 0.35], ring * (0.92 + n * 0.08))
            }
        }
        tile::LEAVES => {
            if n < 0.18 {
                [0, 0, 0, 0]
            } else {
                shade([0.22, 0.52, 0.18], 0.75 + n * 0.4)
            }
        }
        tile::PLANKS => {
            let seam = y % 4 == 3 || (x + (y / 4) * 5) % 16 == 0;
            shade([0.74, 0.58, 0.35], if seam { 0.7 } else { 0.92 + n * 0.1 })
        }
        tile::COAL => ore([0.15, 0.15, 0.17]),
        tile::IRON => ore([0.85, 0.66, 0.52]),
        tile::CRYSTAL => {
            let blob = px_noise(9, t, x / 2, y / 3) > 0.55;
            if blob {
                shade([0.45, 0.95, 1.0], 0.9 + n * 0.2)
            } else {
                stone(n, n2)
            }
        }
        tile::GLASS => {
            let border = x == 0 || y == 0 || x == TILE - 1 || y == TILE - 1;
            let glint = x == y + 3 || x == y + 4;
            if border {
                [220, 235, 240, 255]
            } else if glint && x > 4 && x < 12 {
                [255, 255, 255, 160]
            } else {
                [0, 0, 0, 0]
            }
        }
        tile::BRICKS => {
            let row = y / 4;
            let mortar = y % 4 == 3 || (x + row * 4) % 8 == 7;
            if mortar {
                shade([0.75, 0.75, 0.78], 1.0)
            } else {
                shade([0.55, 0.55, 0.60], 0.88 + n * 0.14)
            }
        }
        tile::BEDROCK => shade([0.25, 0.25, 0.28], 0.5 + n * 0.6),
        tile::LAVA => {
            let w = px_noise(4, t, x / 2, (y + x / 3) / 2);
            shade([1.0, 0.42 + w * 0.35, 0.05], 0.95 + n * 0.1)
        }
        tile::SNOW => shade([0.95, 0.97, 1.0], 0.94 + n * 0.06),
        tile::SNOW_SIDE => {
            if y < 4 {
                shade([0.95, 0.97, 1.0], 0.94 + n * 0.06)
            } else {
                shade([0.53, 0.37, 0.24], grain)
            }
        }
        tile::LAMP => {
            let border = x < 2 || y < 2 || x > TILE - 3 || y > TILE - 3;
            if border {
                shade([0.55, 0.40, 0.22], 1.0)
            } else {
                shade([1.0, 0.86, 0.5], 0.9 + n * 0.15)
            }
        }
        _ => [255, 0, 255, 255],
    }
}

pub fn atlas_pixels() -> Vec<u8> {
    let mut data = vec![0u8; ATLAS_PX * ATLAS_PX * 4];
    for t in 0..tile::COUNT {
        let (tx, ty) = (
            (t as usize % ATLAS_COLS) * TILE,
            (t as usize / ATLAS_COLS) * TILE,
        );
        for y in 0..TILE {
            for x in 0..TILE {
                let p = tile_pixel(t, x, y);
                let i = ((ty + y) * ATLAS_PX + tx + x) * 4;
                data[i..i + 4].copy_from_slice(&p);
            }
        }
    }
    data
}

fn tile_uv(t: u8) -> (f32, f32, f32, f32) {
    let s = 1.0 / ATLAS_COLS as f32;
    let (u, v) = (
        (t as usize % ATLAS_COLS) as f32 * s,
        (t as usize / ATLAS_COLS) as f32 * s,
    );
    let e = 0.0008;
    (u + e, v + e, u + s - e, v + s - e)
}

// ---------------------------------------------------------------- meshing

#[derive(Default)]
pub struct MeshData {
    pub verts: Vec<f32>,
    pub idx: Vec<u32>,
}

impl MeshData {
    fn quad(
        &mut self,
        corners: [[f32; 3]; 4],
        uvs: [[f32; 2]; 4],
        light: [[f32; 3]; 4],
        flip: bool,
    ) {
        let base = (self.verts.len() / 8) as u32;
        for k in 0..4 {
            self.verts.extend_from_slice(&corners[k]);
            self.verts.extend_from_slice(&uvs[k]);
            self.verts.extend_from_slice(&light[k]);
        }
        if flip {
            self.idx
                .extend_from_slice(&[base + 1, base + 2, base + 3, base + 1, base + 3, base]);
        } else {
            self.idx
                .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }
}

const FACE_DIRS: [IVec3; 6] = [
    IVec3::new(0, 1, 0),
    IVec3::new(0, -1, 0),
    IVec3::new(1, 0, 0),
    IVec3::new(-1, 0, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(0, 0, -1),
];

const FACE_SHADE: [f32; 6] = [1.0, 0.5, 0.8, 0.8, 0.66, 0.66];

/// Two tangent axes per face, chosen so (u, v, n) is right-handed and quads wind CCW from outside.
fn face_axes(n: IVec3) -> (IVec3, IVec3) {
    match (n.x, n.y, n.z) {
        (0, 1, 0) => (ivec3(0, 0, 1), ivec3(1, 0, 0)),
        (0, -1, 0) => (ivec3(1, 0, 0), ivec3(0, 0, 1)),
        (1, 0, 0) => (ivec3(0, 1, 0), ivec3(0, 0, 1)),
        (-1, 0, 0) => (ivec3(0, 0, 1), ivec3(0, 1, 0)),
        (0, 0, 1) => (ivec3(1, 0, 0), ivec3(0, 1, 0)),
        _ => (ivec3(0, 1, 0), ivec3(1, 0, 0)),
    }
}

fn face_visible(b: Block, nb: Block) -> bool {
    match b {
        Block::Water => nb != Block::Water && !nb.is_opaque(),
        Block::Lava => nb != Block::Lava && !nb.is_opaque(),
        Block::Glass => nb != Block::Glass && !nb.is_opaque(),
        _ => !nb.is_opaque(),
    }
}

pub fn mesh_chunk(w: &World, cx: i32, cz: i32) -> (MeshData, MeshData) {
    let mut solid = MeshData::default();
    let mut water = MeshData::default();
    for lz in 0..CHUNK {
        for lx in 0..CHUNK {
            let (x, z) = (cx * CHUNK + lx, cz * CHUNK + lz);
            for y in 0..SY {
                let b = w.get(x, y, z);
                if b == Block::Air {
                    continue;
                }
                let p = ivec3(x, y, z);
                let liquid_top = b.is_liquid() && w.get(x, y + 1, z) != b;
                for (fi, n) in FACE_DIRS.iter().enumerate() {
                    let nb = w.get_v(p + *n);
                    if !face_visible(b, nb) {
                        continue;
                    }
                    let out = if b == Block::Water {
                        &mut water
                    } else {
                        &mut solid
                    };
                    emit_face(w, out, b, p, fi, *n, liquid_top);
                }
            }
        }
    }
    (solid, water)
}

fn emit_face(
    w: &World,
    out: &mut MeshData,
    b: Block,
    p: IVec3,
    fi: usize,
    n: IVec3,
    liquid_top: bool,
) {
    let (u, v) = face_axes(n);
    let face_kind = match fi {
        0 => 0,
        1 => 1,
        _ => 2,
    };
    let t = b.tile(face_kind);
    let (u0, v0, u1, v1) = tile_uv(t);
    let origin = p.as_vec3();
    // Face plane lies at the positive side of the normal.
    let plane = origin + Vec3::new(n.x.max(0) as f32, n.y.max(0) as f32, n.z.max(0) as f32);
    let top_drop = if liquid_top { 0.12 } else { 0.0 };
    let emissive = b.emission() > 0;
    let solid_at = |q: IVec3| w.get_v(q).is_opaque();

    let mut corners = [[0.0; 3]; 4];
    let mut uvs = [[0.0; 2]; 4];
    let mut light = [[0.0; 3]; 4];
    let mut ao = [0u8; 4];
    let cs = [(0, 0), (1, 0), (1, 1), (0, 1)];
    for (k, &(cu, cv)) in cs.iter().enumerate() {
        let mut pos = plane;
        // Tangent axes are unit vectors along +x/+y/+z, so offsets are 0 or 1 on those axes.
        pos += u.as_vec3() * cu as f32 + v.as_vec3() * cv as f32;
        if n.x != 0 {
            pos.x = origin.x + n.x.max(0) as f32;
        }
        if n.y != 0 {
            pos.y = origin.y + n.y.max(0) as f32;
        }
        if n.z != 0 {
            pos.z = origin.z + n.z.max(0) as f32;
        }
        if top_drop > 0.0 && (pos.y - (origin.y + 1.0)).abs() < 1e-4 {
            pos.y -= top_drop;
        }
        corners[k] = [pos.x, pos.y, pos.z];

        // Texture: sides map v to world-down so grass edges sit on top.
        let (fu, fv) = if n.y == 0 {
            let along = if u.y == 0 { cu } else { cv };
            let up = if u.y == 0 { cv } else { cu };
            (along as f32, 1.0 - up as f32)
        } else {
            (cu as f32, cv as f32)
        };
        uvs[k] = [u0 + (u1 - u0) * fu, v0 + (v1 - v0) * fv];

        // Ambient occlusion and smooth light from the 4 cells touching this corner.
        let su = if cu == 0 { -u } else { u };
        let sv = if cv == 0 { -v } else { v };
        let front = p + n;
        let s1 = solid_at(front + su);
        let s2 = solid_at(front + sv);
        let c = solid_at(front + su + sv);
        ao[k] = if s1 && s2 {
            0
        } else {
            3 - (s1 as u8 + s2 as u8 + c as u8)
        };
        let mut sky = 0.0;
        let mut glow = 0.0;
        let mut cnt = 0.0;
        for (q, open) in [
            (front, true),
            (front + su, !s1),
            (front + sv, !s2),
            (front + su + sv, !c && !(s1 && s2)),
        ] {
            if open {
                sky += w.sky_light(q.x, q.y, q.z) as f32;
                glow += w.block_light(q.x, q.y, q.z) as f32;
                cnt += 1.0;
            }
        }
        let ao_f = [0.45, 0.64, 0.82, 1.0][ao[k] as usize];
        light[k] = [
            FACE_SHADE[fi] * if emissive { 1.0 } else { ao_f },
            sky / cnt / 15.0,
            if emissive { 1.0 } else { glow / cnt / 15.0 },
        ];
    }
    let flip = ao[0] as u32 + ao[2] as u32 > ao[1] as u32 + ao[3] as u32;
    out.quad(corners, uvs, light, !flip && ao[0] + ao[2] != ao[1] + ao[3]);
}

// ---------------------------------------------------------------- GPU side

const VERTEX: &str = r#"#version 100
attribute vec3 in_pos;
attribute vec2 in_uv;
attribute vec3 in_light;

uniform mat4 mvp;
uniform vec4 cam_pos;
uniform vec4 fog_range;

varying lowp vec2 uv;
varying lowp vec3 light;
varying highp float fog;

void main() {
    gl_Position = mvp * vec4(in_pos, 1.0);
    uv = in_uv;
    light = in_light;
    float d = length(in_pos - cam_pos.xyz);
    fog = clamp((d - fog_range.x) / (fog_range.y - fog_range.x), 0.0, 1.0);
}
"#;

const FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec2 uv;
varying lowp vec3 light;
varying highp float fog;

uniform sampler2D tex;
uniform vec4 fog_color;
uniform vec4 params;

void main() {
    vec4 t = texture2D(tex, uv);
    if (t.a < 0.08) discard;
    float sky = light.y * params.x;
    float glow = light.z;
    float l = max(sky, glow);
    float b = max(pow(0.82, (1.0 - l) * 15.0), 0.03);
    vec3 tint = mix(vec3(1.0), vec3(1.12, 0.97, 0.82), clamp(glow - sky, 0.0, 1.0));
    vec3 c = t.rgb * light.x * b * tint;
    c = mix(c, fog_color.rgb, fog);
    gl_FragColor = vec4(c, t.a * params.y);
}
"#;

#[repr(C)]
struct Uniforms {
    mvp: Mat4,
    cam_pos: Vec4,
    fog_range: Vec4,
    fog_color: Vec4,
    params: Vec4,
}

fn meta() -> ShaderMeta {
    ShaderMeta {
        images: vec!["tex".to_string()],
        uniforms: UniformBlockLayout {
            uniforms: vec![
                UniformDesc::new("mvp", UniformType::Mat4),
                UniformDesc::new("cam_pos", UniformType::Float4),
                UniformDesc::new("fog_range", UniformType::Float4),
                UniformDesc::new("fog_color", UniformType::Float4),
                UniformDesc::new("params", UniformType::Float4),
            ],
        },
    }
}

struct GpuMesh {
    vb: BufferId,
    ib: BufferId,
    count: i32,
}

#[derive(Default)]
struct ChunkGpu {
    solid: Option<GpuMesh>,
    water: Option<GpuMesh>,
}

pub struct TerrainRenderer {
    solid_pipe: Pipeline,
    water_pipe: Pipeline,
    atlas: TextureId,
    chunks: Vec<ChunkGpu>,
}

pub struct FrameParams {
    pub mvp: Mat4,
    pub cam_pos: Vec3,
    pub fog_color: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,
    pub daylight: f32,
}

impl TerrainRenderer {
    pub fn new(ctx: &mut dyn RenderingBackend) -> TerrainRenderer {
        let pixels = atlas_pixels();
        let atlas = ctx.new_texture_from_rgba8(ATLAS_PX as u16, ATLAS_PX as u16, &pixels);
        ctx.texture_set_filter(atlas, FilterMode::Nearest, MipmapFilterMode::None);
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl {
                    vertex: VERTEX,
                    fragment: FRAGMENT,
                },
                meta(),
            )
            .unwrap_or_else(|e| panic!("terrain shader failed to compile: {e:?}"));
        let layout = [BufferLayout::default()];
        let attrs = [
            VertexAttribute::new("in_pos", VertexFormat::Float3),
            VertexAttribute::new("in_uv", VertexFormat::Float2),
            VertexAttribute::new("in_light", VertexFormat::Float3),
        ];
        let solid_pipe = ctx.new_pipeline(
            &layout,
            &attrs,
            shader,
            PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                cull_face: CullFace::Back,
                front_face_order: FrontFaceOrder::CounterClockwise,
                ..Default::default()
            },
        );
        let water_pipe = ctx.new_pipeline(
            &layout,
            &attrs,
            shader,
            PipelineParams {
                depth_test: Comparison::LessOrEqual,
                depth_write: false,
                cull_face: CullFace::Nothing,
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );
        TerrainRenderer {
            solid_pipe,
            water_pipe,
            atlas,
            chunks: (0..CX * CZ).map(|_| ChunkGpu::default()).collect(),
        }
    }

    fn upload(ctx: &mut dyn RenderingBackend, data: &MeshData) -> Option<GpuMesh> {
        if data.idx.is_empty() {
            return None;
        }
        let vb = ctx.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&data.verts),
        );
        let ib = ctx.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(&data.idx),
        );
        Some(GpuMesh {
            vb,
            ib,
            count: data.idx.len() as i32,
        })
    }

    /// Re-mesh up to `budget` dirty chunks (closest first). Returns how many were rebuilt.
    pub fn update(
        &mut self,
        ctx: &mut dyn RenderingBackend,
        w: &mut World,
        near: Vec3,
        budget: usize,
    ) -> usize {
        let mut dirty: Vec<(f32, usize)> = w
            .dirty
            .iter()
            .enumerate()
            .filter(|(_, d)| **d)
            .map(|(i, _)| {
                let (cx, cz) = (i as i32 % CX, i as i32 / CX);
                let c = Vec3::new((cx * CHUNK + 8) as f32, near.y, (cz * CHUNK + 8) as f32);
                (c.distance_squared(near), i)
            })
            .collect();
        dirty.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut built = 0;
        for &(_, i) in dirty.iter().take(budget) {
            let (cx, cz) = (i as i32 % CX, i as i32 / CX);
            let (solid, water) = mesh_chunk(w, cx, cz);
            let slot = &mut self.chunks[i];
            for m in [slot.solid.take(), slot.water.take()].into_iter().flatten() {
                ctx.delete_buffer(m.vb);
                ctx.delete_buffer(m.ib);
            }
            slot.solid = Self::upload(ctx, &solid);
            slot.water = Self::upload(ctx, &water);
            w.dirty[i] = false;
            built += 1;
        }
        built
    }

    fn visible(mvp: &Mat4, cx: i32, cz: i32) -> bool {
        let (x0, z0) = ((cx * CHUNK) as f32, (cz * CHUNK) as f32);
        let (x1, z1) = (x0 + CHUNK as f32, z0 + CHUNK as f32);
        let mut outside = [true; 6];
        for &x in &[x0, x1] {
            for &y in &[0.0, SY as f32] {
                for &z in &[z0, z1] {
                    let c = *mvp * Vec4::new(x, y, z, 1.0);
                    let tests = [
                        c.x < -c.w,
                        c.x > c.w,
                        c.y < -c.w,
                        c.y > c.w,
                        c.z < -c.w,
                        c.z > c.w,
                    ];
                    for k in 0..6 {
                        outside[k] &= tests[k];
                    }
                }
            }
        }
        !outside.iter().any(|o| *o)
    }

    fn uniforms(f: &FrameParams, alpha: f32) -> Uniforms {
        Uniforms {
            mvp: f.mvp,
            cam_pos: f.cam_pos.extend(1.0),
            fog_range: Vec4::new(f.fog_start, f.fog_end, 0.0, 0.0),
            fog_color: Vec4::new(f.fog_color[0], f.fog_color[1], f.fog_color[2], 1.0),
            params: Vec4::new(f.daylight, alpha, 0.0, 0.0),
        }
    }

    fn draw_pass(&self, ctx: &mut dyn RenderingBackend, f: &FrameParams, water: bool) {
        ctx.begin_default_pass(PassAction::Nothing);
        ctx.apply_pipeline(if water {
            &self.water_pipe
        } else {
            &self.solid_pipe
        });
        let u = Self::uniforms(f, 1.0);
        let mut first = true;
        for (i, c) in self.chunks.iter().enumerate() {
            let m = if water { &c.water } else { &c.solid };
            let Some(m) = m else { continue };
            let (cx, cz) = (i as i32 % CX, i as i32 / CX);
            let centre = Vec3::new(
                (cx * CHUNK + 8) as f32,
                f.cam_pos.y,
                (cz * CHUNK + 8) as f32,
            );
            if centre.distance(f.cam_pos) > f.fog_end + 12.0 || !Self::visible(&f.mvp, cx, cz) {
                continue;
            }
            ctx.apply_bindings(&Bindings {
                vertex_buffers: vec![m.vb],
                index_buffer: m.ib,
                images: vec![self.atlas],
            });
            if first {
                ctx.apply_uniforms(UniformsSource::table(&u));
                first = false;
            }
            ctx.draw(0, m.count, 1);
        }
        ctx.end_render_pass();
    }

    pub fn draw_solid(&self, ctx: &mut dyn RenderingBackend, f: &FrameParams) {
        self.draw_pass(ctx, f, false);
    }

    pub fn draw_water(&self, ctx: &mut dyn RenderingBackend, f: &FrameParams) {
        self.draw_pass(ctx, f, true);
    }
}
