//! Procedurally synthesised sound effects and music (no audio files ship with the game).

use crate::noise::Rng;
use std::f32::consts::TAU;

pub const RATE: u32 = 44_100;
const SR: f32 = RATE as f32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sfx {
    Pluck,
    Throw,
    Dig,
    Break,
    Place,
    Bite,
    Hurt,
    SproutDie,
    Deliver,
    Seeds,
    Shard,
    Craft,
    ChomperHit,
    ChomperDie,
    ChomperWake,
    Night,
    Dawn,
    Splash,
    Sizzle,
    Step,
    Click,
}

pub const ALL_SFX: [Sfx; 21] = [
    Sfx::Pluck,
    Sfx::Throw,
    Sfx::Dig,
    Sfx::Break,
    Sfx::Place,
    Sfx::Bite,
    Sfx::Hurt,
    Sfx::SproutDie,
    Sfx::Deliver,
    Sfx::Seeds,
    Sfx::Shard,
    Sfx::Craft,
    Sfx::ChomperHit,
    Sfx::ChomperDie,
    Sfx::ChomperWake,
    Sfx::Night,
    Sfx::Dawn,
    Sfx::Splash,
    Sfx::Sizzle,
    Sfx::Step,
    Sfx::Click,
];

impl Sfx {
    /// How many pitch variants to render, so repeated sounds don't feel robotic.
    pub fn variants(self) -> usize {
        match self {
            Sfx::Pluck | Sfx::Dig | Sfx::Step | Sfx::ChomperHit | Sfx::Throw | Sfx::Place => 3,
            _ => 1,
        }
    }

    /// Shortest gap between two plays of the same sound, in seconds.
    pub fn min_gap(self) -> f32 {
        match self {
            Sfx::Dig => 0.07,
            Sfx::Step => 0.2,
            Sfx::ChomperHit => 0.12,
            Sfx::Splash | Sfx::Sizzle => 0.2,
            _ => 0.04,
        }
    }

    pub fn volume(self) -> f32 {
        match self {
            Sfx::Step => 0.25,
            Sfx::Dig => 0.45,
            Sfx::Click => 0.5,
            Sfx::Night | Sfx::Dawn => 0.6,
            _ => 0.8,
        }
    }
}

// ---------------------------------------------------------------- synthesis primitives

fn osc_sine(phase: f32) -> f32 {
    (phase * TAU).sin()
}

fn osc_tri(phase: f32) -> f32 {
    let p = phase.fract();
    4.0 * (p - 0.5).abs() - 1.0
}

fn osc_square(phase: f32) -> f32 {
    if phase.fract() < 0.5 { 1.0 } else { -1.0 }
}

fn osc_saw(phase: f32) -> f32 {
    2.0 * phase.fract() - 1.0
}

/// Attack/decay envelope: linear attack then exponential-ish decay to zero at `len`.
fn env(t: f32, attack: f32, len: f32) -> f32 {
    if t < 0.0 || t >= len {
        0.0
    } else if t < attack {
        t / attack
    } else {
        let k = 1.0 - (t - attack) / (len - attack).max(1e-4);
        k * k
    }
}

/// A voice whose frequency glides from `f0` to `f1` with a given waveform.
fn sweep(
    buf: &mut [f32],
    start: f32,
    len: f32,
    f0: f32,
    f1: f32,
    wave: fn(f32) -> f32,
    amp: f32,
    attack: f32,
) {
    let s0 = (start * SR) as usize;
    let n = (len * SR) as usize;
    let mut phase = 0.0;
    for i in 0..n {
        let idx = s0 + i;
        if idx >= buf.len() {
            break;
        }
        let t = i as f32 / SR;
        let f = f0 + (f1 - f0) * (t / len);
        phase += f / SR;
        buf[idx] += wave(phase) * amp * env(t, attack, len);
    }
}

/// Noise burst through a one-pole low-pass whose cutoff glides from `c0` to `c1` (0..1).
fn noise(
    buf: &mut [f32],
    rng: &mut Rng,
    start: f32,
    len: f32,
    c0: f32,
    c1: f32,
    amp: f32,
    attack: f32,
) {
    let s0 = (start * SR) as usize;
    let n = (len * SR) as usize;
    let mut lp = 0.0;
    for i in 0..n {
        let idx = s0 + i;
        if idx >= buf.len() {
            break;
        }
        let t = i as f32 / SR;
        let c = c0 + (c1 - c0) * (t / len);
        lp += (rng.range(-1.0, 1.0) - lp) * c;
        buf[idx] += lp * amp * env(t, attack, len);
    }
}

/// High-passed noise (for splashes, sizzles and hi-hats).
fn hiss(buf: &mut [f32], rng: &mut Rng, start: f32, len: f32, amp: f32, attack: f32) {
    let s0 = (start * SR) as usize;
    let n = (len * SR) as usize;
    let mut prev = 0.0;
    for i in 0..n {
        let idx = s0 + i;
        if idx >= buf.len() {
            break;
        }
        let t = i as f32 / SR;
        let x = rng.range(-1.0, 1.0);
        buf[idx] += (x - prev) * 0.5 * amp * env(t, attack, len);
        prev = x;
    }
}

/// Mallet/bell note: sine with a decaying overtone.
fn bell(buf: &mut [f32], start: f32, len: f32, f: f32, amp: f32) {
    let s0 = (start * SR) as usize;
    let n = (len * SR) as usize;
    for i in 0..n {
        let idx = s0 + i;
        if idx >= buf.len() {
            break;
        }
        let t = i as f32 / SR;
        let e = env(t, 0.004, len);
        let over = (-t * 18.0).exp();
        buf[idx] += amp
            * e
            * (osc_sine(f * t)
                + 0.35 * over * osc_sine(f * 4.0 * t)
                + 0.15 * osc_sine(f * 2.0 * t));
    }
}

fn buffer(len: f32) -> Vec<f32> {
    vec![0.0; (len * SR) as usize]
}

fn normalize(buf: &mut [f32], peak: f32) {
    let m = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if m > 1e-6 {
        let k = peak / m;
        for s in buf.iter_mut() {
            *s *= k;
        }
    }
}

fn midi(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

/// Render one sound effect variant (`pitch` scales every frequency).
pub fn render_sfx(s: Sfx, pitch: f32, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let p = pitch;
    let mut b;
    match s {
        Sfx::Pluck => {
            b = buffer(0.22);
            noise(&mut b, &mut rng, 0.0, 0.05, 0.5, 0.1, 0.5, 0.001);
            sweep(
                &mut b,
                0.02,
                0.18,
                320.0 * p,
                980.0 * p,
                osc_sine,
                0.8,
                0.005,
            );
            sweep(
                &mut b,
                0.02,
                0.12,
                640.0 * p,
                1500.0 * p,
                osc_tri,
                0.15,
                0.005,
            );
        }
        Sfx::Throw => {
            b = buffer(0.3);
            noise(&mut b, &mut rng, 0.0, 0.28, 0.05, 0.4, 0.7, 0.08);
            sweep(
                &mut b,
                0.0,
                0.1,
                700.0 * p,
                1000.0 * p,
                osc_square,
                0.08,
                0.005,
            );
        }
        Sfx::Dig => {
            b = buffer(0.09);
            noise(&mut b, &mut rng, 0.0, 0.08, 0.35 * p, 0.05, 0.9, 0.001);
            sweep(&mut b, 0.0, 0.07, 160.0 * p, 90.0 * p, osc_sine, 0.7, 0.002);
        }
        Sfx::Break => {
            b = buffer(0.35);
            noise(&mut b, &mut rng, 0.0, 0.32, 0.6, 0.03, 1.0, 0.002);
            noise(&mut b, &mut rng, 0.05, 0.2, 0.3, 0.02, 0.6, 0.002);
            sweep(&mut b, 0.0, 0.18, 120.0, 55.0, osc_sine, 0.8, 0.002);
        }
        Sfx::Place => {
            b = buffer(0.15);
            sweep(&mut b, 0.0, 0.14, 110.0 * p, 70.0 * p, osc_sine, 1.0, 0.002);
            noise(&mut b, &mut rng, 0.0, 0.04, 0.4, 0.1, 0.5, 0.001);
        }
        Sfx::Bite => {
            b = buffer(0.4);
            for k in 0..2 {
                let t0 = k as f32 * 0.12;
                noise(&mut b, &mut rng, t0, 0.1, 0.5, 0.05, 1.0, 0.002);
                sweep(&mut b, t0, 0.12, 95.0, 55.0, osc_saw, 0.4, 0.002);
            }
        }
        Sfx::Hurt => {
            b = buffer(0.3);
            sweep(&mut b, 0.0, 0.28, 260.0, 110.0, osc_square, 0.35, 0.003);
            sweep(&mut b, 0.0, 0.2, 130.0, 70.0, osc_sine, 0.8, 0.003);
        }
        Sfx::SproutDie => {
            b = buffer(0.9);
            let n = b.len();
            let mut phase = 0.0;
            for i in 0..n {
                let t = i as f32 / SR;
                let f = 900.0 * (1.0 - t / 0.9 * 0.6) * (1.0 + 0.03 * (t * TAU * 7.0).sin());
                phase += f / SR;
                b[i] += osc_tri(phase) * 0.6 * env(t, 0.02, 0.9);
            }
        }
        Sfx::Deliver => {
            b = buffer(0.7);
            for (k, n) in [72.0, 76.0, 79.0].iter().enumerate() {
                bell(&mut b, k as f32 * 0.08, 0.5, midi(*n), 0.5);
            }
        }
        Sfx::Seeds => {
            b = buffer(0.7);
            for k in 0..4 {
                let t0 = k as f32 * 0.12;
                sweep(
                    &mut b,
                    t0,
                    0.12,
                    400.0 + 120.0 * k as f32,
                    1100.0 + 150.0 * k as f32,
                    osc_sine,
                    0.7,
                    0.003,
                );
            }
            bell(&mut b, 0.45, 0.25, midi(84.0), 0.4);
        }
        Sfx::Shard => {
            b = buffer(2.0);
            let notes = [(0.0, 72.0), (0.15, 76.0), (0.3, 79.0), (0.45, 84.0)];
            for (t0, n) in notes {
                bell(&mut b, t0, 1.2, midi(n), 0.45);
                sweep(&mut b, t0, 0.6, midi(n), midi(n), osc_tri, 0.15, 0.01);
            }
            for n in [72.0, 76.0, 79.0, 84.0] {
                sweep(&mut b, 0.6, 1.4, midi(n), midi(n), osc_tri, 0.12, 0.05);
            }
            hiss(&mut b, &mut rng, 0.6, 1.2, 0.08, 0.3);
        }
        Sfx::Craft => {
            b = buffer(0.45);
            bell(&mut b, 0.0, 0.3, midi(79.0), 0.5);
            bell(&mut b, 0.09, 0.35, midi(86.0), 0.5);
        }
        Sfx::ChomperHit => {
            b = buffer(0.15);
            noise(&mut b, &mut rng, 0.0, 0.12, 0.25, 0.05, 0.8, 0.002);
            sweep(
                &mut b,
                0.0,
                0.12,
                200.0 * p,
                120.0 * p,
                osc_sine,
                0.6,
                0.002,
            );
        }
        Sfx::ChomperDie => {
            b = buffer(1.0);
            sweep(&mut b, 0.0, 0.9, 150.0, 50.0, osc_saw, 0.45, 0.02);
            sweep(&mut b, 0.0, 0.9, 75.0, 30.0, osc_sine, 0.6, 0.02);
            noise(&mut b, &mut rng, 0.3, 0.6, 0.2, 0.02, 0.6, 0.05);
        }
        Sfx::ChomperWake => {
            b = buffer(0.6);
            let n = b.len();
            let mut phase = 0.0;
            for i in 0..n {
                let t = i as f32 / SR;
                let f = 85.0 + 25.0 * (t * TAU * 9.0).sin();
                phase += f / SR;
                b[i] += osc_saw(phase) * 0.5 * env(t, 0.05, 0.6);
            }
            noise(&mut b, &mut rng, 0.0, 0.5, 0.08, 0.03, 0.4, 0.05);
        }
        Sfx::Night => {
            b = buffer(2.5);
            for n in [57.0, 60.0, 64.0] {
                sweep(&mut b, 0.0, 2.4, midi(n), midi(n), osc_tri, 0.25, 0.6);
            }
            bell(&mut b, 0.3, 1.5, midi(81.0), 0.15);
        }
        Sfx::Dawn => {
            b = buffer(2.0);
            for (k, n) in [60.0, 64.0, 67.0, 72.0].iter().enumerate() {
                bell(&mut b, k as f32 * 0.18, 1.3, midi(*n), 0.35);
            }
            for n in [60.0, 67.0] {
                sweep(&mut b, 0.2, 1.7, midi(n), midi(n), osc_sine, 0.2, 0.4);
            }
        }
        Sfx::Splash => {
            b = buffer(0.45);
            hiss(&mut b, &mut rng, 0.0, 0.4, 1.0, 0.01);
            noise(&mut b, &mut rng, 0.0, 0.25, 0.3, 0.05, 0.4, 0.005);
        }
        Sfx::Sizzle => {
            b = buffer(0.5);
            hiss(&mut b, &mut rng, 0.0, 0.5, 0.8, 0.02);
            sweep(&mut b, 0.0, 0.3, 900.0, 300.0, osc_tri, 0.2, 0.01);
        }
        Sfx::Step => {
            b = buffer(0.07);
            noise(&mut b, &mut rng, 0.0, 0.06, 0.15 * p, 0.03, 1.0, 0.002);
        }
        Sfx::Click => {
            b = buffer(0.04);
            sweep(&mut b, 0.0, 0.03, 1800.0, 1400.0, osc_sine, 0.6, 0.001);
        }
    }
    normalize(&mut b, 0.9);
    b
}

// ---------------------------------------------------------------- music

struct Song {
    bpm: f32,
    bars: usize,
    /// Root MIDI note of each chord (two bars each) and whether it is minor.
    chords: &'static [(f32, bool)],
    scale: &'static [f32],
    melody_density: f32,
    melody_octave: f32,
    shaker: bool,
}

const DAY: Song = Song {
    bpm: 96.0,
    bars: 16,
    chords: &[(60.0, false), (57.0, true), (53.0, false), (55.0, false)],
    scale: &[0.0, 2.0, 4.0, 7.0, 9.0],
    melody_density: 0.62,
    melody_octave: 72.0,
    shaker: true,
};

const NIGHT: Song = Song {
    bpm: 70.0,
    bars: 16,
    chords: &[(57.0, true), (53.0, false), (50.0, true), (52.0, false)],
    scale: &[0.0, 3.0, 5.0, 7.0, 10.0],
    melody_density: 0.35,
    melody_octave: 69.0,
    shaker: false,
};

/// Adds a voice to a looping buffer (samples past the end wrap to the start).
fn add_wrapped(buf: &mut [f32], start: usize, voice: &[f32]) {
    let n = buf.len();
    for (i, v) in voice.iter().enumerate() {
        buf[(start + i) % n] += *v;
    }
}

fn note_voice(len: f32, f: f32, kind: u8, amp: f32) -> Vec<f32> {
    let n = (len * SR) as usize;
    let mut v = vec![0.0; n];
    for (i, s) in v.iter_mut().enumerate() {
        let t = i as f32 / SR;
        *s = match kind {
            // Soft pad: slow attack, gentle detuned sines.
            0 => {
                let e = (t / 0.4).min(1.0) * ((len - t) / 0.6).clamp(0.0, 1.0);
                amp * e
                    * (osc_sine(f * t) + 0.5 * osc_sine(f * 1.003 * t) + 0.2 * osc_tri(f * 2.0 * t))
            }
            // Round bass.
            1 => amp * env(t, 0.01, len) * (osc_tri(f * t) + 0.3 * osc_sine(f * 0.5 * t)),
            // Marimba-like lead.
            _ => {
                let e = env(t, 0.003, len);
                let over = (-t * 14.0).exp();
                amp * e
                    * (osc_sine(f * t)
                        + 0.4 * over * osc_sine(f * 4.0 * t)
                        + 0.1 * osc_tri(f * 2.0 * t))
            }
        };
    }
    v
}

fn render_song(song: &Song, seed: u64) -> Vec<f32> {
    let mut rng = Rng::new(seed);
    let beat = 60.0 / song.bpm;
    let bar = beat * 4.0;
    let total = bar * song.bars as f32;
    let mut buf = vec![0.0f32; (total * SR) as usize];
    let at = |t: f32| (t * SR) as usize;

    // Chords: pad + bass.
    for b in 0..song.bars {
        let (root, minor) = song.chords[(b / 2) % song.chords.len()];
        let third = if minor { 3.0 } else { 4.0 };
        let t0 = b as f32 * bar;
        if b % 2 == 0 {
            for n in [root, root + third, root + 7.0] {
                let v = note_voice(bar * 2.0 + 0.5, midi(n), 0, 0.07);
                add_wrapped(&mut buf, at(t0), &v);
            }
        }
        for beat_i in [0.0, 2.0] {
            let n = if beat_i == 0.0 {
                root - 12.0
            } else {
                root - 5.0
            };
            let v = note_voice(beat * 1.6, midi(n), 1, 0.22);
            add_wrapped(&mut buf, at(t0 + beat_i * beat), &v);
        }
    }

    // Melody: a wandering walk over the scale, phrased in 2-bar sentences that repeat with
    // variation so the tune is memorable.
    let eighth = beat / 2.0;
    let steps_per_phrase = 16;
    let phrases = song.bars / 2;
    let mut motif: Vec<Option<f32>> = Vec::new();
    let mut idx: i32 = 2;
    for _ in 0..steps_per_phrase {
        if rng.chance(song.melody_density) {
            idx = (idx + rng.irange(-2, 3)).clamp(0, song.scale.len() as i32 * 2 - 1);
            let oct = (idx / song.scale.len() as i32) as f32 * 12.0;
            motif.push(Some(
                song.scale[(idx % song.scale.len() as i32) as usize] + oct,
            ));
        } else {
            motif.push(None);
        }
    }
    for ph in 0..phrases {
        for (s, note) in motif.iter().enumerate() {
            let Some(n) = note else { continue };
            // Every other phrase varies a few notes.
            let n = if ph % 2 == 1 && rng.chance(0.3) {
                n + [2.0, -2.0, 3.0][rng.irange(0, 3) as usize]
            } else {
                *n
            };
            let t0 = ph as f32 * bar * 2.0 + s as f32 * eighth;
            let len = eighth * if rng.chance(0.3) { 3.0 } else { 1.8 };
            let v = note_voice(len, midi(song.melody_octave + n), 2, 0.16);
            add_wrapped(&mut buf, at(t0), &v);
        }
    }

    // Light percussion.
    let mut prng = Rng::new(seed ^ 0xD00D);
    for b in 0..song.bars {
        let t0 = b as f32 * bar;
        let mut kick = vec![0.0; (0.18 * SR) as usize];
        sweep(&mut kick, 0.0, 0.16, 110.0, 45.0, osc_sine, 0.35, 0.002);
        add_wrapped(&mut buf, at(t0), &kick);
        add_wrapped(&mut buf, at(t0 + 2.0 * beat), &kick);
        if song.shaker {
            for e in 0..8 {
                let mut h = vec![0.0; (0.05 * SR) as usize];
                let amp = if e % 2 == 1 { 0.05 } else { 0.025 };
                hiss(&mut h, &mut prng, 0.0, 0.045, amp, 0.002);
                add_wrapped(&mut buf, at(t0 + e as f32 * eighth), &h);
            }
        }
    }
    normalize(&mut buf, 0.8);
    buf
}

pub fn render_music(night: bool) -> Vec<f32> {
    if night {
        render_song(&NIGHT, 0x5EED_0002)
    } else {
        render_song(&DAY, 0x5EED_0001)
    }
}

/// The whistle is a seamless one-second loop: the vibrato completes whole cycles.
pub fn render_whistle() -> Vec<f32> {
    let n = RATE as usize;
    let mut b = vec![0.0; n];
    for (i, s) in b.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let phase = 1760.0 * t - (40.0 / (TAU * 6.0)) * (TAU * 6.0 * t).cos();
        *s = 0.45 * osc_sine(phase) + 0.08 * osc_sine(2.0 * phase);
    }
    b
}

/// Encode mono samples as a 16-bit PCM WAV file.
pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

// ---------------------------------------------------------------- playback

use crate::controls::Settings;
use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
};
use std::collections::HashMap;

pub struct AudioPlayer {
    sfx: HashMap<Sfx, Vec<Sound>>,
    last_played: HashMap<Sfx, f64>,
    round_robin: usize,
    whistle: Option<Sound>,
    whistling: bool,
    day: Option<Sound>,
    night: Option<Sound>,
    music_on: bool,
    volumes: (f32, f32),
    night_mix: f32,
}

const MUSIC_VOLUME: f32 = 0.4;

async fn load(samples: &[f32]) -> Option<Sound> {
    load_sound_from_bytes(&wav_bytes(samples)).await.ok()
}

impl AudioPlayer {
    /// Synthesises and uploads every sound. Takes a moment, so call it behind a loading screen.
    pub async fn load() -> AudioPlayer {
        let mut sfx = HashMap::new();
        for s in ALL_SFX {
            let mut v = Vec::new();
            for k in 0..s.variants() {
                let pitch = [1.0, 1.12, 0.89][k % 3];
                if let Some(snd) = load(&render_sfx(s, pitch, 17 + k as u64)).await {
                    v.push(snd);
                }
            }
            sfx.insert(s, v);
        }
        AudioPlayer {
            sfx,
            last_played: HashMap::new(),
            round_robin: 0,
            whistle: load(&render_whistle()).await,
            whistling: false,
            day: load(&render_music(false)).await,
            night: load(&render_music(true)).await,
            music_on: false,
            volumes: (0.0, 0.0),
            night_mix: 0.0,
        }
    }

    /// Play a one-shot effect at `volume` (already including distance falloff).
    pub fn play(&mut self, s: Sfx, volume: f32, settings: &Settings) {
        if !settings.sounds || volume <= 0.01 {
            return;
        }
        let now = macroquad::time::get_time();
        if self
            .last_played
            .get(&s)
            .is_some_and(|t| now - t < s.min_gap() as f64)
        {
            return;
        }
        let Some(variants) = self.sfx.get(&s).filter(|v| !v.is_empty()) else {
            return;
        };
        self.last_played.insert(s, now);
        self.round_robin = self.round_robin.wrapping_add(1);
        let snd = &variants[self.round_robin % variants.len()];
        play_sound(
            snd,
            PlaySoundParams {
                looped: false,
                volume: (volume * s.volume()).min(1.0),
            },
        );
    }

    pub fn set_whistle(&mut self, on: bool, settings: &Settings) {
        let on = on && settings.sounds;
        if on == self.whistling {
            return;
        }
        self.whistling = on;
        if let Some(w) = &self.whistle {
            if on {
                play_sound(
                    w,
                    PlaySoundParams {
                        looped: true,
                        volume: 0.35,
                    },
                );
            } else {
                stop_sound(w);
            }
        }
    }

    /// Keep both loops running and crossfade between the day and night tunes.
    pub fn update_music(&mut self, night: bool, dt: f32, settings: &Settings) {
        let (Some(day), Some(nightm)) = (&self.day, &self.night) else {
            return;
        };
        if settings.music && !self.music_on {
            play_sound(
                day,
                PlaySoundParams {
                    looped: true,
                    volume: 0.0,
                },
            );
            play_sound(
                nightm,
                PlaySoundParams {
                    looped: true,
                    volume: 0.0,
                },
            );
            self.music_on = true;
            self.volumes = (0.0, 0.0);
        } else if !settings.music && self.music_on {
            stop_sound(day);
            stop_sound(nightm);
            self.music_on = false;
            return;
        }
        if !self.music_on {
            return;
        }
        let target = if night { 1.0 } else { 0.0 };
        self.night_mix += (target - self.night_mix).clamp(-dt / 4.0, dt / 4.0);
        let want = (
            MUSIC_VOLUME * (1.0 - self.night_mix),
            MUSIC_VOLUME * self.night_mix,
        );
        if (want.0 - self.volumes.0).abs() > 0.005 || (want.1 - self.volumes.1).abs() > 0.005 {
            set_sound_volume(day, want.0);
            set_sound_volume(nightm, want.1);
            self.volumes = want;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_renders_audible_and_clean() {
        for s in ALL_SFX {
            for v in 0..s.variants() {
                let b = render_sfx(s, 1.0 + v as f32 * 0.1, v as u64);
                assert!(!b.is_empty(), "{s:?} empty");
                let peak = b.iter().fold(0.0f32, |m, x| m.max(x.abs()));
                assert!(peak > 0.5 && peak <= 1.0, "{s:?} peak {peak}");
                assert!(b.iter().all(|x| x.is_finite()), "{s:?} has NaN");
            }
        }
    }

    #[test]
    fn music_loops_are_long_and_bounded() {
        for night in [false, true] {
            let m = render_music(night);
            let secs = m.len() as f32 / SR;
            assert!(secs > 30.0, "loop only {secs}s");
            assert!(m.iter().all(|x| x.is_finite() && x.abs() <= 1.0));
        }
    }

    #[test]
    fn whistle_loop_is_seamless() {
        let w = render_whistle();
        // The wrap-around step must look like any other step between neighbouring samples.
        let max_step = w
            .windows(2)
            .map(|p| (p[1] - p[0]).abs())
            .fold(0.0f32, f32::max);
        let jump = (w[0] - w[w.len() - 1]).abs();
        assert!(
            jump <= max_step * 1.01,
            "click at loop point: {jump} vs {max_step}"
        );
    }

    #[test]
    fn wav_header_is_valid() {
        let w = wav_bytes(&[0.0, 0.5, -0.5]);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(u32::from_le_bytes(w[24..28].try_into().unwrap()), RATE);
    }
}
