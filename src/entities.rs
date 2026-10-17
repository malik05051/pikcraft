//! Creature and item data types.

use crate::block::Block;
use crate::physics::Body;
use macroquad::math::{IVec3, Vec3};

pub const C_RED: u8 = 0;
pub const C_YELLOW: u8 = 1;
pub const C_BLUE: u8 = 2;
pub const COLOR_NAMES: [&str; 3] = ["Red", "Yellow", "Blue"];
pub const COLOR_RGB: [[f32; 3]; 3] = [[0.92, 0.18, 0.14], [1.0, 0.86, 0.15], [0.2, 0.42, 0.98]];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Task {
    None,
    Block(IVec3),
    Item(u32),
    Enemy(u32),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SproutState {
    /// Planted in the ground, waiting to be plucked.
    Buried,
    Idle,
    Follow,
    Thrown(Task),
    Mine(IVec3),
    /// Walking to an item it wants to carry.
    GoCarry(u32),
    Carry(u32),
    /// Running to (or latched onto, when `latched`) an enemy.
    Attack {
        enemy: u32,
        latched: bool,
    },
    Dead,
}

pub struct Sprout {
    pub id: u32,
    pub color: u8,
    pub body: Body,
    pub state: SproutState,
    pub yaw: f32,
    pub anim: f32,
    /// 0 leaf, 1 bud, 2 flower — grows with age and makes the sprout faster.
    pub maturity: u8,
    pub age: f32,
    pub drown: f32,
    pub burn: f32,
    pub timer: f32,
    pub think: f32,
    /// Offset around the carried item / latched enemy.
    pub slot: Vec3,
    pub panic: f32,
}

impl Sprout {
    pub fn new(id: u32, color: u8, pos: Vec3, state: SproutState) -> Sprout {
        Sprout {
            id,
            color,
            body: Body::new(pos, 0.18, 0.75),
            state,
            yaw: 0.0,
            anim: (id as f32 * 1.37) % std::f32::consts::TAU,
            maturity: 0,
            age: 0.0,
            drown: 0.0,
            burn: 0.0,
            timer: 0.0,
            think: (id % 7) as f32 * 0.07,
            slot: Vec3::ZERO,
            panic: 0.0,
        }
    }

    pub fn alive(&self) -> bool {
        self.state != SproutState::Dead
    }

    pub fn in_field(&self) -> bool {
        !matches!(self.state, SproutState::Buried | SproutState::Dead)
    }

    pub fn speed(&self) -> f32 {
        [4.2, 5.0, 5.8][self.maturity as usize]
    }

    /// Digging speed in "hand-seconds per second".
    pub fn dig_power(&self) -> f32 {
        let base = if self.color == C_YELLOW { 2.0 } else { 1.0 };
        base * [1.0, 1.15, 1.3][self.maturity as usize]
    }

    pub fn attack_power(&self) -> f32 {
        let base = if self.color == C_RED { 10.0 } else { 5.0 };
        base * [1.0, 1.15, 1.3][self.maturity as usize]
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EnemyState {
    Sleeping,
    Wander,
    Chase,
    Bite,
    Shake,
}

pub struct Enemy {
    pub id: u32,
    pub body: Body,
    pub hp: f32,
    pub max_hp: f32,
    pub yaw: f32,
    pub state: EnemyState,
    pub timer: f32,
    pub bite_cd: f32,
    pub shake_cd: f32,
    pub size: f32,
    pub wander_target: Vec3,
    /// Spawned at night; dissolves at sunrise.
    pub nocturnal: bool,
    pub hurt: f32,
    pub mouth: f32,
    pub alive: bool,
}

impl Enemy {
    pub fn new(id: u32, pos: Vec3, size: f32, nocturnal: bool) -> Enemy {
        let hp = 70.0 * size * size;
        Enemy {
            id,
            body: Body::new(pos, 0.55 * size, 1.1 * size),
            hp,
            max_hp: hp,
            yaw: (id as f32 * 2.1) % std::f32::consts::TAU,
            state: if nocturnal {
                EnemyState::Wander
            } else {
                EnemyState::Sleeping
            },
            timer: 0.0,
            bite_cd: 1.0,
            shake_cd: 4.0,
            size,
            wander_target: pos,
            nocturnal,
            hurt: 0.0,
            mouth: 0.0,
            alive: true,
        }
    }

    pub fn radius(&self) -> f32 {
        0.6 * self.size
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ItemKind {
    Block(Block),
    Carcass { seeds: u32, size: f32 },
    Shard,
}

pub struct Item {
    pub id: u32,
    pub kind: ItemKind,
    pub body: Body,
    pub carriers: Vec<u32>,
    pub path: Vec<IVec3>,
    pub path_i: usize,
    pub repath: f32,
    pub stuck: f32,
    pub dig: f32,
    pub alive: bool,
    pub spin: f32,
    pub no_path: bool,
    pub age: f32,
}

impl Item {
    pub fn new(id: u32, kind: ItemKind, pos: Vec3) -> Item {
        let hw = match kind {
            ItemKind::Block(_) => 0.15,
            _ => 0.3,
        };
        Item {
            id,
            kind,
            body: Body::new(pos, hw, 0.4),
            carriers: Vec::new(),
            path: Vec::new(),
            path_i: 0,
            repath: 0.0,
            stuck: 0.0,
            dig: 0.0,
            alive: true,
            spin: (id as f32 * 0.7) % std::f32::consts::TAU,
            no_path: false,
            age: 0.0,
        }
    }

    /// Sprouts needed to lift it.
    pub fn weight(&self) -> usize {
        match self.kind {
            ItemKind::Block(_) => 1,
            ItemKind::Carcass { size, .. } => (3.0 * size * size).round() as usize,
            ItemKind::Shard => 5,
        }
    }

    pub fn max_carriers(&self) -> usize {
        self.weight() * 2
    }

    pub fn label(&self) -> String {
        match self.kind {
            ItemKind::Block(b) => b.name().to_string(),
            ItemKind::Carcass { .. } => "Chomper".to_string(),
            ItemKind::Shard => "Star Shard".to_string(),
        }
    }
}

pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub color: [f32; 3],
    pub life: f32,
    pub size: f32,
    pub gravity: f32,
}
