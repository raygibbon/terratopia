use super::{biome::Biome, profile::Profile, seed::hash};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Flora {
    Fern,
    Fungus,
    Toxic,
    Medicinal,
}
impl Flora {
    pub fn name(self) -> &'static str {
        match self {
            Self::Fern => "Ribbon fern",
            Self::Fungus => "Glow fungus",
            Self::Toxic => "Venom frond",
            Self::Medicinal => "Medicinal starleaf",
        }
    }
    pub fn glyph(self) -> char {
        match self {
            Self::Fern => '&',
            Self::Fungus => '"',
            Self::Toxic => '!',
            Self::Medicinal => '+',
        }
    }
    pub fn colour(self) -> u8 {
        match self {
            Self::Fern => 10,
            Self::Fungus => 14,
            Self::Toxic => 13,
            Self::Medicinal => 11,
        }
    }
    pub fn at(seed: u64, x: i64, y: i64, biome: Biome, profile: Profile) -> Option<Self> {
        if !biome.passable() {
            return None;
        }
        let h = hash(seed ^ 0x464c4f5241, x.div_euclid(12), y.div_euclid(12));
        if h % 100 >= u64::from(profile.vegetation) {
            return None;
        }
        let ax = 2 + ((h >> 8) % 8) as i64;
        let ay = 2 + ((h >> 16) % 8) as i64;
        let dx = x.rem_euclid(12) - ax;
        let dy = y.rem_euclid(12) - ay;
        if dx * dx + dy * dy > 2 {
            return None;
        }
        match (h >> 24) % 4 {
            0 if matches!(biome, Biome::Grassland | Biome::Forest | Biome::Wasteland) => {
                Some(Self::Fern)
            }
            1 if matches!(biome, Biome::Forest | Biome::Frozen | Biome::Wasteland) => {
                Some(Self::Fungus)
            }
            2 if profile.toxicity > 20 || biome == Biome::Wasteland => Some(Self::Toxic),
            3 if matches!(biome, Biome::Grassland | Biome::Forest | Biome::Frozen) => {
                Some(Self::Medicinal)
            }
            _ => None,
        }
    }
}
