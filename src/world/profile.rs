//! Integer planetary parameters; separate salted seed stream from terrain.
use super::seed::mix;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Archetype {
    Temperate,
    Frozen,
    Desert,
    Toxic,
    Irradiated,
    Volcanic,
    Barren,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    pub archetype: Archetype,
    pub oxygen: u8,
    pub temperature: i16,
    pub variation: u8,
    pub gravity: u16,
    pub radiation: u8,
    pub toxicity: u8,
    pub water: u8,
    pub vegetation: u8,
    pub wildlife: u8,
    pub minerals: u16,
    pub legacy: bool,
}
impl Archetype {
    pub fn name(self) -> &'static str {
        match self {
            Self::Temperate => "Temperate terrestrial",
            Self::Frozen => "Frozen wasteland",
            Self::Desert => "Desert world",
            Self::Toxic => "Toxic jungle",
            Self::Irradiated => "Irradiated planet",
            Self::Volcanic => "Volcanic world",
            Self::Barren => "Barren rocky planet",
        }
    }
}
impl Profile {
    pub fn legacy() -> Self {
        Self {
            archetype: Archetype::Temperate,
            oxygen: 0,
            temperature: 18,
            variation: 20,
            gravity: 100,
            radiation: 0,
            toxicity: 0,
            water: 50,
            vegetation: 50,
            wildlife: 50,
            minerals: 100,
            legacy: true,
        }
    }
    pub fn generate(seed: u64) -> Self {
        let h = mix(seed ^ 0x504c414e4554);
        let mut p = Self::legacy();
        p.legacy = false;
        p.archetype = match h % 7 {
            0 => Archetype::Temperate,
            1 => Archetype::Frozen,
            2 => Archetype::Desert,
            3 => Archetype::Toxic,
            4 => Archetype::Irradiated,
            5 => Archetype::Volcanic,
            _ => Archetype::Barren,
        };
        let (o, t, v, r, to, w, plants, life, m) = match p.archetype {
            Archetype::Temperate => (22, 18, 16, 5, 0, 65, 80, 80, 100),
            Archetype::Frozen => (8, -18, 24, 10, 5, 35, 25, 25, 110),
            Archetype::Desert => (5, 36, 32, 15, 5, 12, 15, 30, 130),
            Archetype::Toxic => (14, 28, 14, 15, 65, 75, 95, 85, 90),
            Archetype::Irradiated => (4, 20, 22, 70, 20, 30, 25, 25, 150),
            Archetype::Volcanic => (1, 48, 24, 40, 45, 15, 10, 20, 180),
            Archetype::Barren => (0, 2, 36, 20, 5, 5, 8, 15, 160),
        };
        p.oxygen = o;
        p.temperature = t + ((h >> 8) % 9) as i16 - 4;
        p.variation = v;
        p.radiation = r;
        p.toxicity = to;
        p.water = w;
        p.vegetation = plants;
        p.wildlife = life;
        p.minerals = m;
        p.gravity = 60 + ((h >> 16) % 121) as u16;
        p
    }
    pub fn atmosphere(self) -> &'static str {
        if self.toxicity > 40 {
            "Nitrogen / corrosive vapour"
        } else if self.oxygen >= 18 {
            "Nitrogen / oxygen"
        } else if self.oxygen == 0 {
            "Thin argon / carbon dioxide"
        } else {
            "Carbon dioxide / trace oxygen"
        }
    }
    pub fn breathable(self) -> bool {
        self.oxygen >= 18 && self.toxicity < 15
    }
    pub fn climate(self, e: i64, t: i64, m: i64) -> (i64, i64, i64) {
        if self.legacy {
            return (e, t, m);
        }
        (
            (e + (50 - i64::from(self.water)) * 240).clamp(0, 65535),
            (t + i64::from(self.temperature - 18) * 650).clamp(0, 65535),
            (m + (i64::from(self.vegetation) - 50) * 280).clamp(0, 65535),
        )
    }
}
