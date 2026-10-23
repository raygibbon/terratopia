//! Regional weather is a pure function of seed, biome and active simulation time.
use crate::world::{
    biome::Biome,
    profile::{Archetype, Profile},
    seed::hash,
};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Condition {
    #[default]
    Clear,
    Rain,
    Dust,
    Toxic,
    Snow,
    Radiation,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Weather {
    pub current: Condition,
    pub next: Condition,
    pub severity: u8,
    pub warning: Option<u8>,
}
impl Condition {
    pub fn name(self) -> &'static str {
        match self {
            Self::Clear => "Clear",
            Self::Rain => "Rain",
            Self::Dust => "Dust storm",
            Self::Toxic => "Toxic storm",
            Self::Snow => "Snowstorm",
            Self::Radiation => "Radiation storm",
        }
    }
    pub fn dangerous(self) -> bool {
        !matches!(self, Self::Clear | Self::Rain)
    }
}
impl Weather {
    pub fn at(seed: u64, profile: Profile, biome: Biome, x: i64, y: i64, ticks: u64) -> Self {
        let seconds = ticks / 4;
        let epoch = seconds / 64;
        let phase = seconds % 64;
        let h = hash(
            seed ^ 0x57454154484552 ^ epoch,
            x.div_euclid(64),
            y.div_euclid(64),
        );
        // Mild, wet regions get longer clear spells; exposed and hostile
        // planets have more frequent storms. Keep this stream independent of AI.
        let clear_slots: u64 = match profile.archetype {
            Archetype::Temperate => match biome {
                Biome::Forest | Biome::Grassland => 4,
                _ => 2,
            },
            Archetype::Frozen | Archetype::Desert | Archetype::Barren => 2,
            Archetype::Toxic | Archetype::Irradiated => 1,
            Archetype::Volcanic => 0,
        };
        let next = if h % 10 < clear_slots {
            Condition::Clear
        } else {
            match profile.archetype {
                Archetype::Toxic => Condition::Toxic,
                Archetype::Irradiated => Condition::Radiation,
                Archetype::Frozen => Condition::Snow,
                Archetype::Desert | Archetype::Barren => Condition::Dust,
                Archetype::Volcanic => {
                    if h & 1 == 0 {
                        Condition::Dust
                    } else {
                        Condition::Radiation
                    }
                }
                Archetype::Temperate => match biome {
                    Biome::Frozen | Biome::Mountains => Condition::Snow,
                    Biome::Desert => Condition::Dust,
                    Biome::Wasteland => Condition::Toxic,
                    _ => Condition::Rain,
                },
            }
        };
        if phase < 40 {
            return Self {
                current: Condition::Clear,
                next,
                severity: 0,
                warning: (phase >= 32 && next != Condition::Clear).then_some((40 - phase) as u8),
            };
        }
        let peak_limit = match (profile.archetype, biome) {
            (Archetype::Temperate, Biome::Forest | Biome::Grassland) => 2,
            _ => 3,
        };
        let peak = 1 + ((h >> 8) % peak_limit) as u8;
        let severity = if next == Condition::Clear {
            0
        } else {
            peak.min(1 + ((phase - 40).min(63 - phase) / 4) as u8)
        };
        Self {
            current: next,
            next,
            severity,
            warning: None,
        }
    }
    pub fn temperature(self) -> i16 {
        i16::from(self.severity)
            * match self.current {
                Condition::Snow => -8,
                Condition::Rain => -3,
                Condition::Dust => 3,
                _ => 0,
            }
    }
    pub fn radius(self, scan: bool) -> i64 {
        let normal = if scan { 20 } else { 7 };
        normal.min(match self.current {
            Condition::Clear => 20,
            Condition::Rain => {
                if scan {
                    16
                } else {
                    5
                }
            }
            Condition::Dust => {
                if scan {
                    8
                } else {
                    4
                }
            }
            Condition::Snow => {
                if scan {
                    10
                } else {
                    4
                }
            }
            _ => {
                if scan {
                    12
                } else {
                    5
                }
            }
        })
    }
}
