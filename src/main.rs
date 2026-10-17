#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
//! PikCraft: a voxel sandbox where a squad of plant creatures digs, fights and carries for you.

mod audio;
mod block;
mod controls;
mod draw;
mod entities;
mod game;
mod noise;
mod physics;
mod render;
mod world;

use audio::{AudioPlayer, Sfx};
use controls::{Layout, Settings, digit_pressed, typed_chars};
use draw::*;
use entities::*;
use game::*;
use macroquad::miniquad::conf::Icon;
use macroquad::prelude::*;
use render::{FrameParams, TerrainRenderer};
use std::io::Write;

const SENSITIVITY: f32 = 0.0024;

fn icon_pixels<const N: usize>(size: usize) -> [u8; N] {
    // A red sprout head with a leaf, drawn on a 16x16 grid and scaled up.
    const ART: [&str; 16] = [
        "........gg......",
        ".......gGGg.....",
        "........gG......",
        "........g.......",
        "........g.......",
        "...rrrrrrrrrr...",
        "..rrrrrrrrrrrr..",
        "..rrrrrrrrrrrr..",
        "..rrwkrrrrwkrr..",
        "..rrwkrrrrwkrr..",
        "..rrrrrrrrrrrr..",
        "..rrrrrppprrrr..",
        "..rrrrrrrrrrrr..",
        "...rrrrrrrrrr...",
        "....dd....dd....",
        "....dd....dd....",
    ];
    let mut out = [0u8; N];
    let scale = size / 16;
    for y in 0..size {
        for x in 0..size {
            let c = ART[y / scale].as_bytes()[x / scale];
            let rgba: [u8; 4] = match c {
                b'g' => [70, 150, 50, 255],
                b'G' => [100, 200, 70, 255],
                b'r' => [230, 50, 40, 255],
                b'w' => [255, 255, 255, 255],
                b'k' => [20, 20, 20, 255],
                b'p' => [140, 20, 30, 255],
                b'd' => [150, 30, 25, 255],
                _ => [0, 0, 0, 0],
            };
            let i = (y * size + x) * 4;
            out[i..i + 4].copy_from_slice(&rgba);
        }
    }
    out
}

fn window_conf() -> Conf {
    Conf {
        window_title: "PikCraft".to_owned(),
        window_width: 1280,
        window_height: 720,
        high_dpi: false,
        sample_count: 1,
        icon: Some(Icon {
            small: icon_pixels::<1024>(16),
            medium: icon_pixels::<4096>(32),
            big: icon_pixels::<16384>(64),
        }),
        ..Default::default()
    }
}

struct Options {
    seed: Option<u32>,
    selftest: Option<String>,
}

fn parse_args() -> Options {
    let mut o = Options {
        seed: None,
        selftest: None,
    };
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" if i + 1 < args.len() => {
                o.seed = args[i + 1].parse().ok();
                i += 1;
            }
            "--selftest" if i + 1 < args.len() => {
                o.selftest = Some(args[i + 1].clone());
                i += 1;
            }
            "--help" | "-h" => {
                // Release builds on Windows have no console, so never panic on a closed stdout.
                let mut out = std::io::stdout();
                let _ = writeln!(
                    out,
                    "PikCraft {}\n\nUsage: pikcraft [--seed N]\n",
                    env!("CARGO_PKG_VERSION")
                );
                for l in help_lines(Layout::detect()) {
                    let _ = writeln!(out, "{l}");
                }
                std::process::exit(0);
            }
            "--export-audio" if i + 1 < args.len() => {
                // Developer helper: write every synthesised sound to WAV files and exit.
                let dir = std::path::PathBuf::from(&args[i + 1]);
                let _ = std::fs::create_dir_all(&dir);
                for s in audio::ALL_SFX {
                    let wav = audio::wav_bytes(&audio::render_sfx(s, 1.0, 17));
                    let _ = std::fs::write(dir.join(format!("{s:?}.wav").to_lowercase()), wav);
                }
                let _ = std::fs::write(
                    dir.join("whistle.wav"),
                    audio::wav_bytes(&audio::render_whistle()),
                );
                let _ = std::fs::write(
                    dir.join("music_day.wav"),
                    audio::wav_bytes(&audio::render_music(false)),
                );
                let _ = std::fs::write(
                    dir.join("music_night.wav"),
                    audio::wav_bytes(&audio::render_music(true)),
                );
                std::process::exit(0);
            }
            "--version" | "-V" => {
                let _ = writeln!(std::io::stdout(), "PikCraft {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    o
}

fn random_seed() -> u32 {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1);
    (t as u64 ^ (t >> 64) as u64) as u32 % 1_000_000
}

fn read_input(grabbed: bool, skip_look: bool, layout: Layout, chars: &[char]) -> Input {
    let mut i = Input::default();
    let key = |k| layout.down(k);
    i.forward = key(layout.forward()) as i32 as f32 - key(KeyCode::S) as i32 as f32;
    i.strafe = key(KeyCode::D) as i32 as f32 - key(layout.left()) as i32 as f32;
    i.jump = is_key_down(KeyCode::Space);
    i.sprint = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::LeftControl);
    if !grabbed {
        return i;
    }
    if !skip_look {
        let d = mouse_delta_px();
        i.look = (d.x * SENSITIVITY, d.y * SENSITIVITY);
    }
    i.primary = is_mouse_button_down(MouseButton::Left);
    i.primary_pressed = is_mouse_button_pressed(MouseButton::Left);
    i.secondary = is_mouse_button_down(MouseButton::Right);
    i.secondary_pressed = is_mouse_button_pressed(MouseButton::Right);
    i.throw = layout.pressed(KeyCode::F) || is_mouse_button_pressed(MouseButton::Middle);
    i.whistle = key(KeyCode::R);
    i.pluck = key(KeyCode::E);
    i.dismiss = layout.pressed(KeyCode::X);
    i.cycle_color = is_key_pressed(KeyCode::Tab);
    i.slot = digit_pressed(chars);
    i.scroll = mouse_wheel().1;
    i
}

thread_local! {
    static LAST_MOUSE: std::cell::Cell<Option<Vec2>> = const { std::cell::Cell::new(None) };
}

fn mouse_delta_px() -> Vec2 {
    let now: Vec2 = mouse_position().into();
    let last = LAST_MOUSE.with(|m| m.replace(Some(now)));
    last.map(|l| now - l).unwrap_or(Vec2::ZERO)
}

fn reset_mouse() {
    let now: Vec2 = mouse_position().into();
    LAST_MOUSE.with(|m| m.set(Some(now)));
}

fn draw_title(seed: u32, t: f32, settings: &Settings) {
    let (w, h) = (screen_width(), screen_height());
    clear_background(Color::new(0.45, 0.68, 0.95, 1.0));
    // Rolling blocky hills.
    let cols = (w / 32.0) as i32 + 2;
    for c in 0..cols {
        let x = c as f32 * 32.0;
        let hh = ((c as f32 * 0.35 + t * 0.2).sin() * 2.0 + (c as f32 * 0.13).cos() * 2.0 + 6.0)
            .round()
            * 32.0;
        draw_rectangle(x, h - hh, 32.0, hh, Color::new(0.53, 0.37, 0.24, 1.0));
        draw_rectangle(x, h - hh, 32.0, 10.0, Color::new(0.36, 0.66, 0.26, 1.0));
        draw_rectangle_lines(x, h - hh, 32.0, hh, 1.0, Color::new(0.0, 0.0, 0.0, 0.12));
    }
    // A parade of sprouts.
    for k in 0..9 {
        let x = ((t * 60.0 + k as f32 * 70.0) % (w + 140.0)) - 70.0;
        let c = COLOR_RGB[k % 3];
        let y = h - 70.0 + ((t * 9.0 + k as f32).sin().abs() * -8.0);
        draw_rectangle(x, y, 36.0, 34.0, Color::new(c[0], c[1], c[2], 1.0));
        draw_rectangle(x + 22.0, y + 9.0, 5.0, 8.0, BLACK);
        draw_rectangle(x + 30.0, y + 9.0, 5.0, 8.0, BLACK);
        draw_rectangle(
            x + 16.0,
            y - 18.0,
            3.0,
            18.0,
            Color::new(0.3, 0.6, 0.2, 1.0),
        );
        draw_rectangle(
            x + 16.0,
            y - 22.0,
            14.0,
            6.0,
            Color::new(0.35, 0.75, 0.25, 1.0),
        );
        draw_rectangle(
            x + 8.0,
            y + 34.0,
            8.0,
            10.0,
            Color::new(c[0] * 0.6, c[1] * 0.6, c[2] * 0.6, 1.0),
        );
        draw_rectangle(
            x + 22.0,
            y + 34.0,
            8.0,
            10.0,
            Color::new(c[0] * 0.6, c[1] * 0.6, c[2] * 0.6, 1.0),
        );
    }
    // Logo.
    let title = "PikCraft";
    let size = 120.0;
    let m = measure_text(title, None, size as u16, 1.0);
    let mut x = (w - m.width) / 2.0;
    let y = h * 0.3;
    let letters = [
        [0.92, 0.18, 0.14],
        [1.0, 0.86, 0.15],
        [0.2, 0.42, 0.98],
        [0.36, 0.66, 0.26],
    ];
    for (i, ch) in title.chars().enumerate() {
        let s = ch.to_string();
        let cm = measure_text(&s, None, size as u16, 1.0);
        let c = letters[i % 4];
        let bob = (t * 3.0 + i as f32 * 0.6).sin() * 4.0;
        draw_text(
            &s,
            x + 5.0,
            y + 5.0 + bob,
            size,
            Color::new(0.0, 0.0, 0.0, 0.45),
        );
        draw_text(&s, x, y + bob, size, Color::new(c[0], c[1], c[2], 1.0));
        x += cm.width;
    }
    text_center(
        "Dig, build and lead a squad of sprouts to recover 5 Star Shards",
        w / 2.0,
        y + 50.0,
        26.0,
        WHITE,
    );
    let blink = if (t * 2.0) as i32 % 2 == 0 { 1.0 } else { 0.6 };
    text_center(
        "Click or press Enter to start",
        w / 2.0,
        h * 0.55,
        34.0,
        Color::new(1.0, 1.0, 0.8, blink),
    );
    text_center(
        &format!("World seed: {}   ([N] new seed)", seed),
        w / 2.0,
        h * 0.55 + 40.0,
        22.0,
        WHITE,
    );
    text_center(
        &format!(
            "Keyboard: {} (move {})   [L] switch layout",
            settings.layout.name(),
            settings.layout.move_keys()
        ),
        w / 2.0,
        h * 0.55 + 72.0,
        22.0,
        Color::new(1.0, 1.0, 0.75, 1.0),
    );
    let onoff = |b: bool| if b { "on" } else { "off" };
    text_center(
        &format!(
            "[M] music {}   [B] sounds {}   F1 in game for controls",
            onoff(settings.music),
            onoff(settings.sounds)
        ),
        w / 2.0,
        h * 0.55 + 100.0,
        20.0,
        Color::new(1.0, 1.0, 1.0, 0.8),
    );
}

/// Layout / music / sound toggles shared by the title screen, the pause menu and the game.
fn handle_setting_keys(settings: &mut Settings, audio: &mut AudioPlayer) {
    let layout = settings.layout;
    let mut changed = false;
    if is_key_pressed(KeyCode::L) {
        settings.layout = layout.toggled();
        changed = true;
    }
    if layout.pressed(KeyCode::M) {
        settings.music = !settings.music;
        changed = true;
    }
    if layout.pressed(KeyCode::B) {
        settings.sounds = !settings.sounds;
        changed = true;
    }
    if changed {
        settings.save();
        audio.play(Sfx::Click, 1.0, settings);
    }
}

async fn loading_frame(msg: &str) {
    clear_background(Color::new(0.08, 0.12, 0.08, 1.0));
    text_center(
        msg,
        screen_width() / 2.0,
        screen_height() / 2.0,
        36.0,
        WHITE,
    );
    next_frame().await;
}

fn main() {
    let opts = parse_args();
    macroquad::Window::from_config(window_conf(), run(opts));
}

async fn run(opts: Options) {
    let mut seed = opts.seed.unwrap_or_else(random_seed);
    let mut settings = Settings::load();
    loading_frame("Tuning the sprouts' voices...").await;
    let mut audio = AudioPlayer::load().await;

    // Title screen.
    if opts.selftest.is_none() {
        let start = get_time();
        loop {
            let t = (get_time() - start) as f32;
            audio.update_music(false, get_frame_time(), &settings);
            draw_title(seed, t, &settings);
            handle_setting_keys(&mut settings, &mut audio);
            if settings.layout.pressed(KeyCode::N) {
                seed = random_seed();
                audio.play(Sfx::Click, 1.0, &settings);
            }
            if is_key_pressed(KeyCode::Enter)
                || is_key_pressed(KeyCode::Space)
                || is_mouse_button_pressed(MouseButton::Left)
            {
                break;
            }
            if is_key_pressed(KeyCode::Escape) && t > 0.5 {
                return;
            }
            next_frame().await;
        }
    }

    loading_frame("Growing the world...").await;
    let mut game = Game::new(seed);
    let hud = Hud::new();
    let mut terrain = {
        let gl = unsafe { get_internal_gl() };
        TerrainRenderer::new(gl.quad_context)
    };
    // Build every chunk mesh up front, with a progress bar.
    loop {
        let built = {
            let gl = unsafe { get_internal_gl() };
            terrain.update(gl.quad_context, &mut game.world, game.player.body.pos, 12)
        };
        let left = game.world.dirty.iter().filter(|d| **d).count();
        let total = game.world.dirty.len();
        clear_background(Color::new(0.08, 0.12, 0.08, 1.0));
        let (w, h) = (screen_width(), screen_height());
        text_center("Planting sprouts...", w / 2.0, h / 2.0 - 20.0, 36.0, WHITE);
        draw_rectangle(
            w / 2.0 - 200.0,
            h / 2.0,
            400.0,
            16.0,
            Color::new(0.0, 0.0, 0.0, 0.5),
        );
        draw_rectangle(
            w / 2.0 - 200.0,
            h / 2.0,
            400.0 * (1.0 - left as f32 / total as f32),
            16.0,
            Color::new(0.4, 0.85, 0.3, 1.0),
        );
        next_frame().await;
        if built == 0 || left == 0 {
            break;
        }
    }

    let mut selftest = opts.selftest.map(SelfTest::new);
    let mut frame: u64 = 0;
    let mut paused = false;
    let mut crafting = false;
    let mut help = selftest.is_none();
    let mut help_auto_close = 25.0f32;
    let mut debug_overlay = false;
    let mut grabbed = false;
    let mut skip_look = true;

    loop {
        frame += 1;
        // The self-test uses a fixed step so its scripted checks don't depend on frame rate.
        let dt = if selftest.is_some() {
            1.0 / 30.0
        } else {
            get_frame_time().min(0.05)
        };
        let time = get_time() as f32;
        let chars = typed_chars();
        let layout = settings.layout;

        // ---------------------------------------------------------------- UI state
        let want_grab = !paused && !crafting && selftest.is_none();
        if want_grab != grabbed {
            grabbed = want_grab;
            set_cursor_grab(grabbed);
            show_mouse(!grabbed);
            reset_mouse();
            skip_look = true;
        }
        if is_key_pressed(KeyCode::Escape) {
            if crafting {
                crafting = false;
            } else {
                paused = !paused;
            }
        }
        if is_key_pressed(KeyCode::F3) {
            debug_overlay = !debug_overlay;
        }
        if is_key_pressed(KeyCode::F1) {
            help = !help;
            help_auto_close = 0.0;
        }
        if help && help_auto_close > 0.0 {
            help_auto_close -= dt;
            if help_auto_close <= 0.0 {
                help = false;
            }
        }
        if layout.pressed(KeyCode::C) && !paused {
            crafting = !crafting;
            help = false;
            audio.play(Sfx::Click, 1.0, &settings);
        }
        if crafting {
            if let Some(n) = digit_pressed(&chars).filter(|n| *n < RECIPES.len()) {
                game.craft(n);
            }
        }
        if selftest.is_none() {
            handle_setting_keys(&mut settings, &mut audio);
        }

        // ---------------------------------------------------------------- simulate
        let mut input = if grabbed {
            read_input(true, skip_look, layout, &chars)
        } else {
            Input::default()
        };
        if grabbed {
            skip_look = false;
        } else {
            mouse_delta_px();
        }
        if let Some(st) = selftest.as_mut() {
            input = st.input(&mut game, frame);
        }
        if !paused {
            if crafting {
                // Keep the world ticking while the menu is open, but without player control.
                game.update(&Input::default(), dt);
            } else {
                game.update(&input, dt);
            }
        }
        if help && (input.primary_pressed || input.throw) {
            help = false;
        }

        // ---------------------------------------------------------------- audio
        let events = std::mem::take(&mut game.sfx);
        let ear = game.player.eye();
        for (sfx, at) in &events {
            let volume = match at {
                None => 1.0,
                Some(p) => (1.0 - p.distance(ear) / 36.0).clamp(0.0, 1.0).powf(1.5),
            };
            audio.play(*sfx, volume, &settings);
        }
        audio.set_whistle(game.player.whistle > 0.0 && !paused, &settings);
        audio.update_music(game.is_night(), dt, &settings);
        if let Some(st) = selftest.as_mut() {
            st.heard(&events);
        }

        // ---------------------------------------------------------------- render
        let sky = game.sky_color();
        let sky = if game.player.body.head_in_water {
            [0.1, 0.22, 0.45]
        } else {
            sky
        };
        clear_background(Color::new(sky[0], sky[1], sky[2], 1.0));
        let eye = game.player.eye();
        let cam = Camera3D {
            position: eye,
            target: eye + game.player.forward(),
            up: Vec3::Y,
            fovy: 72f32.to_radians(),
            z_near: 0.05,
            z_far: 420.0,
            ..Default::default()
        };
        let mvp = cam.matrix();
        let (fog_start, fog_end) = if game.player.body.head_in_water {
            (2.0, 22.0)
        } else {
            (55.0, 118.0)
        };
        let fp = FrameParams {
            mvp,
            cam_pos: eye,
            fog_color: sky,
            fog_start,
            fog_end,
            daylight: game.daylight(),
        };
        {
            let gl = unsafe { get_internal_gl() };
            terrain.update(gl.quad_context, &mut game.world, game.player.body.pos, 4);
        }
        set_camera(&cam);
        {
            let mut gl = unsafe { get_internal_gl() };
            gl.flush();
            terrain.draw_solid(gl.quad_context, &fp);
        }
        let command_mode = HOTBAR[game.player.slot].is_none();
        let target = game.look_target(if command_mode { 30.0 } else { 5.0 }, true);
        let preview = if command_mode {
            match target {
                Target::Block { pos, normal, .. } => {
                    Some((pos + normal).as_vec3() + vec3(0.5, 0.0, 0.5))
                }
                Target::None => None,
                Target::Enemy { point, .. }
                | Target::Item { point, .. }
                | Target::Sprout { point, .. } => Some(point),
            }
        } else {
            None
        };
        draw_entities(&game, time, preview);
        if let Target::Block { pos, dist, .. } = target {
            if !command_mode && dist < 5.0 {
                draw_block_cursor(&game, pos);
            }
        }
        {
            let mut gl = unsafe { get_internal_gl() };
            gl.flush();
            terrain.draw_water(gl.quad_context, &fp);
        }
        draw_beams(&game);
        draw_hand(&game);

        set_default_camera();
        let hud_target = if command_mode {
            game.look_target(5.0, true)
        } else {
            target
        };
        draw_hud(&game, &hud, &mvp, &hud_target);
        if crafting {
            show_mouse(true);
            let rects = draw_crafting(&game, &hud, None);
            let m: Vec2 = mouse_position().into();
            let hover = rects.iter().position(|r| r.contains(m));
            if hover.is_some() {
                draw_crafting(&game, &hud, hover);
            }
            if is_mouse_button_pressed(MouseButton::Left) {
                if let Some(i) = hover {
                    game.craft(i);
                }
            }
        }
        if help {
            draw_help(layout);
        }
        if debug_overlay {
            let p = game.player.body.pos;
            let lines = [
                format!("PikCraft {}   {} fps", env!("CARGO_PKG_VERSION"), get_fps()),
                format!(
                    "XYZ {:.1} {:.1} {:.1}   facing {:.0} deg",
                    p.x,
                    p.y,
                    p.z,
                    game.player.yaw.to_degrees().rem_euclid(360.0)
                ),
                format!(
                    "Keyboard {} (move {})   music {}   sounds {}",
                    layout.name(),
                    layout.move_keys(),
                    if settings.music { "on" } else { "off" },
                    if settings.sounds { "on" } else { "off" }
                ),
                format!(
                    "Seed {}   sprouts {}   chompers {}",
                    game.world.seed,
                    game.sprouts.len(),
                    game.enemies.len()
                ),
            ];
            draw_rectangle(10.0, 60.0, 520.0, 112.0, Color::new(0.0, 0.0, 0.0, 0.55));
            for (k, l) in lines.iter().enumerate() {
                text_shadow(l, 18.0, 84.0 + k as f32 * 24.0, 20.0, WHITE);
            }
        }
        if let Some(t) = game.won_at {
            if game.elapsed - t < 12.0 {
                let (w, h) = (screen_width(), screen_height());
                draw_panel(w / 2.0 - 330.0, h * 0.2, 660.0, 150.0);
                text_center(
                    "ALL STAR SHARDS RECOVERED!",
                    w / 2.0,
                    h * 0.2 + 55.0,
                    40.0,
                    Color::new(1.0, 0.9, 0.3, 1.0),
                );
                let mins = (t / 60.0) as u32;
                let secs = (t % 60.0) as u32;
                text_center(
                    &format!(
                        "Time {}:{:02}   Plucked {}   Lost {}   Chompers {}",
                        mins, secs, game.stats.plucked, game.stats.lost, game.stats.chompers
                    ),
                    w / 2.0,
                    h * 0.2 + 95.0,
                    24.0,
                    WHITE,
                );
                text_center(
                    "Keep playing as long as you like!",
                    w / 2.0,
                    h * 0.2 + 128.0,
                    20.0,
                    Color::new(1.0, 1.0, 1.0, 0.8),
                );
            }
        }
        if paused {
            let (w, h) = (screen_width(), screen_height());
            draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.5));
            text_center("Paused", w / 2.0, h / 2.0 - 30.0, 60.0, WHITE);
            text_center(
                "Esc resume   Q quit   F1 help",
                w / 2.0,
                h / 2.0 + 20.0,
                26.0,
                WHITE,
            );
            let onoff = |b: bool| if b { "on" } else { "off" };
            text_center(
                &format!(
                    "[M] music {}   [B] sounds {}   [L] keyboard {}",
                    onoff(settings.music),
                    onoff(settings.sounds),
                    settings.layout.name()
                ),
                w / 2.0,
                h / 2.0 + 90.0,
                22.0,
                Color::new(1.0, 1.0, 0.75, 1.0),
            );
            text_center(
                &format!("World seed {}", game.world.seed),
                w / 2.0,
                h / 2.0 + 56.0,
                20.0,
                Color::new(1.0, 1.0, 1.0, 0.7),
            );
            if layout.pressed(KeyCode::Q) {
                break;
            }
        }
        if !paused && !crafting && !help && game.elapsed < 6.0 {
            text_center(
                "Press F1 for controls",
                screen_width() / 2.0,
                screen_height() * 0.3,
                22.0,
                Color::new(1.0, 1.0, 1.0, 0.8),
            );
        }

        if let Some(st) = selftest.as_mut() {
            if st.after_frame(&game, frame) {
                std::process::exit(if st.failures.is_empty() { 0 } else { 1 });
            }
        }
        next_frame().await;
    }
}

/// Scripted run used by CI and development: plays a few scenarios and saves screenshots.
struct SelfTest {
    dir: String,
    log: Vec<String>,
    failures: Vec<String>,
    sounds: std::collections::HashSet<Sfx>,
    base_yaw: f32,
    carried: usize,
}

impl SelfTest {
    fn new(dir: String) -> SelfTest {
        let _ = std::fs::create_dir_all(&dir);
        SelfTest {
            dir,
            log: Vec::new(),
            failures: Vec::new(),
            sounds: std::collections::HashSet::new(),
            base_yaw: 0.0,
            carried: 0,
        }
    }

    fn heard(&mut self, events: &[(Sfx, Option<Vec3>)]) {
        self.sounds.extend(events.iter().map(|(s, _)| *s));
    }

    fn check(&mut self, ok: bool, what: &str) {
        self.log
            .push(format!("{} {}", if ok { "ok  " } else { "FAIL" }, what));
        if !ok {
            self.failures.push(what.to_string());
        }
    }

    fn input(&mut self, g: &mut Game, frame: u64) -> Input {
        let mut i = Input::default();
        let nest = g.nest;
        let pluck_nearby = |g: &mut Game, i: &mut Input| {
            let target = g
                .sprouts
                .iter()
                .filter(|s| s.state == SproutState::Buried)
                .min_by(|a, b| {
                    a.body
                        .pos
                        .distance(nest)
                        .total_cmp(&b.body.pos.distance(nest))
                })
                .map(|s| s.body.pos);
            if let Some(t) = target {
                if t.distance(nest) < 8.0 {
                    g.player.body.pos = t + vec3(0.9, 0.0, 0.0);
                    g.player.body.vel = Vec3::ZERO;
                    i.pluck = true;
                }
            }
        };
        match frame {
            2 => {
                g.player.inv[block::Block::Planks as usize] = 20;
                g.player.inv[block::Block::GlowLamp as usize] = 5;
                g.player.inv[block::Block::Stone as usize] = 12;
            }
            3..=90 if frame % 6 == 3 => pluck_nearby(g, &mut i),
            100 => {
                for c in [C_YELLOW, C_BLUE, C_RED] {
                    let n = g.plant_seeds(c, 3);
                    self.log.push(format!("planted {n} seeds"));
                }
            }
            101..=200 if frame % 6 == 3 => pluck_nearby(g, &mut i),
            205 => {
                g.player.body.pos = nest + vec3(0.0, 0.0, -4.5);
                g.player.yaw = -std::f32::consts::FRAC_PI_2;
                g.player.pitch = -0.2;
                self.base_yaw = g.player.yaw;
            }
            206..=229 => i.forward = 1.0,
            230 => {
                // Turn around to look at the squad trotting behind us.
                g.player.yaw = self.base_yaw + std::f32::consts::PI;
                g.player.pitch = -0.35;
            }
            250 => {
                g.player.yaw = self.base_yaw;
                g.player.pitch = -0.75;
                g.player.slot = 0;
            }
            251..=300 => i.throw = frame % 5 == 0,
            310 => g.player.pitch = -0.5,
            420..=440 => i.whistle = true,
            450 => {
                // Stage a fight on the open meadow next to the nest.
                let ep = nest + vec3(5.5, 0.0, 3.5);
                let id = g.new_id();
                g.enemies.push(Enemy::new(id, ep, 1.0, false));
                g.player.body.pos = nest + vec3(-1.5, 0.0, 3.5);
                for s in g
                    .sprouts
                    .iter_mut()
                    .filter(|s| s.state == SproutState::Follow)
                {
                    s.body.pos = g.player.body.pos + vec3(-1.0, 0.3, 0.0);
                }
                let d = ep + vec3(0.0, 0.6, 0.0) - g.player.eye();
                g.player.yaw = d.z.atan2(d.x);
                g.player.pitch = d.y.atan2(vec2(d.x, d.z).length());
            }
            455..=520 => i.throw = frame % 6 == 0,
            700 => {
                g.time_of_day = 0.66;
                g.player.slot = 7;
                g.player.body.pos = nest + vec3(-6.0, 0.0, -6.0);
                g.player.body.pos.y =
                    g.world.surface_y(nest.x as i32 - 6, nest.z as i32 - 6) as f32 + 1.0;
                g.player.yaw = 0.8;
                g.player.pitch = -0.6;
            }
            705 => i.secondary = true,
            706 => g.player.pitch = -0.15,
            _ => {}
        }
        i
    }

    fn after_frame(&mut self, g: &Game, frame: u64) -> bool {
        let dir = self.dir.clone();
        let shot = |name: &str| {
            get_screen_data().export_png(&format!("{dir}/{name}.png"));
        };
        let states = |g: &Game| {
            let mut m = std::collections::BTreeMap::new();
            for s in &g.sprouts {
                let k = match s.state {
                    SproutState::Buried => "buried",
                    SproutState::Idle => "idle",
                    SproutState::Follow => "follow",
                    SproutState::Thrown(_) => "thrown",
                    SproutState::Mine(_) => "mine",
                    SproutState::GoCarry(_) => "gocarry",
                    SproutState::Carry(_) => "carry",
                    SproutState::Attack { .. } => "attack",
                    SproutState::Dead => "dead",
                };
                *m.entry(k).or_insert(0) += 1;
            }
            format!("{m:?}")
        };
        match frame {
            60 => shot("01_start"),
            240 => {
                self.log.push(format!(
                    "f240 squad {:?} states {}",
                    g.squad_counts(),
                    states(g)
                ));
                let (squad, _) = g.squad_counts();
                self.check(
                    squad.iter().sum::<u32>() >= 9,
                    "plucked sprouts follow the player",
                );
                shot("02_squad");
            }
            330 => {
                self.log
                    .push(format!("f330 states {} items {}", states(g), g.items.len()));
                shot("03_digging");
            }
            301..=410 => {
                let busy = g
                    .sprouts
                    .iter()
                    .filter(|s| matches!(s.state, SproutState::Carry(_) | SproutState::Mine(_)))
                    .count();
                self.carried = self.carried.max(busy);
                if frame == 410 {
                    self.log.push(format!(
                        "f410 states {} dirt {}",
                        states(g),
                        g.player.has(block::Block::Dirt)
                    ));
                    self.check(self.carried >= 3, "thrown sprouts dig and carry blocks");
                }
            }
            445 => {
                self.log
                    .push(format!("f445 after whistle states {}", states(g)));
                let (squad, field) = g.squad_counts();
                self.check(
                    squad.iter().sum::<u32>() + 2 >= field.iter().sum::<u32>(),
                    "whistle recalls the squad",
                );
            }
            530 => {
                let e: Vec<String> = g
                    .enemies
                    .iter()
                    .take(3)
                    .map(|e| format!("{:.0}/{:.0} {:?}", e.hp, e.max_hp, e.state))
                    .collect();
                self.log
                    .push(format!("f530 states {} enemies {:?}", states(g), e));
                shot("04_attack");
            }
            690 => {
                let e: Vec<String> = g
                    .enemies
                    .iter()
                    .take(3)
                    .map(|e| format!("{:.0}/{:.0} {:?}", e.hp, e.max_hp, e.state))
                    .collect();
                self.log.push(format!(
                    "f690 states {} enemies {:?} lost {} chompers {}",
                    states(g),
                    e,
                    g.stats.lost,
                    g.stats.chompers
                ));
                self.check(g.stats.chompers >= 1, "sprouts defeat a chomper");
                self.check(g.stats.delivered >= 1, "carried loot reaches the nest");
                shot("05_fight");
            }
            740 => shot("06_night"),
            750 => {
                self.check(g.is_night(), "day/night cycle");
                let needed = [
                    Sfx::Pluck,
                    Sfx::Throw,
                    Sfx::Dig,
                    Sfx::Break,
                    Sfx::ChomperDie,
                    Sfx::Deliver,
                    Sfx::Night,
                ];
                let missing: Vec<_> = needed.iter().filter(|s| !self.sounds.contains(s)).collect();
                self.check(
                    missing.is_empty(),
                    &format!("gameplay sounds fire (missing {missing:?})"),
                );
                self.log.push(format!("fps {}", get_fps()));
                self.log.push(if self.failures.is_empty() {
                    "RESULT: PASS".into()
                } else {
                    format!("RESULT: FAIL {:?}", self.failures)
                });
                let _ = std::fs::write(format!("{}/log.txt", self.dir), self.log.join("\n"));
                return true;
            }
            _ => {}
        }
        false
    }
}
