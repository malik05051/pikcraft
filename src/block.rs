//! Block types and their static properties.

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
#[repr(u8)]
pub enum Block {
    Air = 0,
    Grass,
    Dirt,
    Stone,
    Sand,
    Water,
    Log,
    Leaves,
    Planks,
    CoalOre,
    IronOre,
    CrystalOre,
    Glass,
    Bricks,
    Bedrock,
    Lava,
    Snow,
    GlowLamp,
    Gravel,
}

pub const ALL_BLOCKS: [Block; 19] = [
    Block::Air,
    Block::Grass,
    Block::Dirt,
    Block::Stone,
    Block::Sand,
    Block::Water,
    Block::Log,
    Block::Leaves,
    Block::Planks,
    Block::CoalOre,
    Block::IronOre,
    Block::CrystalOre,
    Block::Glass,
    Block::Bricks,
    Block::Bedrock,
    Block::Lava,
    Block::Snow,
    Block::GlowLamp,
    Block::Gravel,
];

/// Texture atlas tile indices.
pub mod tile {
    pub const GRASS_TOP: u8 = 0;
    pub const GRASS_SIDE: u8 = 1;
    pub const DIRT: u8 = 2;
    pub const STONE: u8 = 3;
    pub const SAND: u8 = 4;
    pub const WATER: u8 = 5;
    pub const LOG_SIDE: u8 = 6;
    pub const LOG_TOP: u8 = 7;
    pub const LEAVES: u8 = 8;
    pub const PLANKS: u8 = 9;
    pub const COAL: u8 = 10;
    pub const IRON: u8 = 11;
    pub const CRYSTAL: u8 = 12;
    pub const GLASS: u8 = 13;
    pub const BRICKS: u8 = 14;
    pub const BEDROCK: u8 = 15;
    pub const LAVA: u8 = 16;
    pub const SNOW: u8 = 17;
    pub const SNOW_SIDE: u8 = 18;
    pub const LAMP: u8 = 19;
    pub const GRAVEL: u8 = 20;
    pub const COUNT: u8 = 21;
}

impl Block {
    pub fn from_u8(v: u8) -> Block {
        ALL_BLOCKS.get(v as usize).copied().unwrap_or(Block::Air)
    }

    pub fn name(self) -> &'static str {
        match self {
            Block::Air => "Air",
            Block::Grass => "Grass",
            Block::Dirt => "Dirt",
            Block::Stone => "Stone",
            Block::Sand => "Sand",
            Block::Water => "Water",
            Block::Log => "Log",
            Block::Leaves => "Leaves",
            Block::Planks => "Planks",
            Block::CoalOre => "Coal",
            Block::IronOre => "Iron",
            Block::CrystalOre => "Crystal",
            Block::Glass => "Glass",
            Block::Bricks => "Bricks",
            Block::Bedrock => "Bedrock",
            Block::Lava => "Lava",
            Block::Snow => "Snow",
            Block::GlowLamp => "Glow Lamp",
            Block::Gravel => "Gravel",
        }
    }

    /// Blocks movement.
    pub fn is_solid(self) -> bool {
        !matches!(self, Block::Air | Block::Water | Block::Lava)
    }

    /// Fully hides neighbouring faces and blocks light.
    pub fn is_opaque(self) -> bool {
        self.is_solid() && !matches!(self, Block::Leaves | Block::Glass)
    }

    pub fn is_liquid(self) -> bool {
        matches!(self, Block::Water | Block::Lava)
    }

    /// Light emitted (0..=15).
    pub fn emission(self) -> u8 {
        match self {
            Block::Lava => 14,
            Block::GlowLamp => 15,
            Block::CrystalOre => 9,
            _ => 0,
        }
    }

    /// Seconds for a bare hand (or one sprout) to break the block. `None` = unbreakable.
    pub fn hardness(self) -> Option<f32> {
        match self {
            Block::Air | Block::Water | Block::Lava | Block::Bedrock => None,
            Block::Leaves | Block::Glass => Some(0.35),
            Block::Grass | Block::Dirt | Block::Sand | Block::Snow | Block::Gravel => Some(0.75),
            Block::Log | Block::Planks => Some(1.8),
            Block::GlowLamp => Some(0.6),
            Block::Stone | Block::Bricks => Some(3.0),
            Block::CoalOre => Some(3.5),
            Block::IronOre => Some(4.5),
            Block::CrystalOre => Some(5.0),
        }
    }

    /// Pickaxes speed these up a lot; other blocks only slightly.
    pub fn is_rocky(self) -> bool {
        matches!(
            self,
            Block::Stone | Block::Bricks | Block::CoalOre | Block::IronOre | Block::CrystalOre
        )
    }

    /// What lands in the inventory when this block is broken.
    pub fn drop(self) -> Block {
        match self {
            Block::Grass => Block::Dirt,
            Block::Glass => Block::Air,
            b => b,
        }
    }

    /// Tile for faces: 0 = top, 1 = bottom, 2 = side.
    pub fn tile(self, face: usize) -> u8 {
        use tile::*;
        match self {
            Block::Grass => [GRASS_TOP, DIRT, GRASS_SIDE][face],
            Block::Dirt => DIRT,
            Block::Stone => STONE,
            Block::Sand => SAND,
            Block::Water => WATER,
            Block::Log => [LOG_TOP, LOG_TOP, LOG_SIDE][face],
            Block::Leaves => LEAVES,
            Block::Planks => PLANKS,
            Block::CoalOre => COAL,
            Block::IronOre => IRON,
            Block::CrystalOre => CRYSTAL,
            Block::Glass => GLASS,
            Block::Bricks => BRICKS,
            Block::Bedrock => BEDROCK,
            Block::Lava => LAVA,
            Block::Snow => [SNOW, DIRT, SNOW_SIDE][face],
            Block::GlowLamp => LAMP,
            Block::Gravel => GRAVEL,
            Block::Air => STONE,
        }
    }

    /// Representative colour used for icons, particles and dropped items.
    pub fn color(self) -> [f32; 3] {
        match self {
            Block::Grass => [0.36, 0.66, 0.26],
            Block::Dirt => [0.53, 0.37, 0.24],
            Block::Stone => [0.52, 0.52, 0.54],
            Block::Sand => [0.86, 0.80, 0.56],
            Block::Water => [0.20, 0.40, 0.85],
            Block::Log => [0.45, 0.32, 0.18],
            Block::Leaves => [0.22, 0.52, 0.18],
            Block::Planks => [0.74, 0.58, 0.35],
            Block::CoalOre => [0.22, 0.22, 0.24],
            Block::IronOre => [0.78, 0.62, 0.50],
            Block::CrystalOre => [0.45, 0.90, 0.95],
            Block::Glass => [0.80, 0.92, 0.96],
            Block::Bricks => [0.62, 0.62, 0.66],
            Block::Bedrock => [0.18, 0.18, 0.20],
            Block::Lava => [1.00, 0.45, 0.08],
            Block::Snow => [0.95, 0.97, 1.00],
            Block::GlowLamp => [1.00, 0.88, 0.55],
            Block::Gravel => [0.50, 0.47, 0.45],
            Block::Air => [1.0, 1.0, 1.0],
        }
    }
}
