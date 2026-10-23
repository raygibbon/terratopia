//! Coordinate-owned sites use a 24-cell grid, independent of 32-cell chunks.
use super::seed::hash;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Resource {
    Iron,
    Copper,
    Silica,
    Carbon,
    Crystal,
}
impl Resource {
    pub fn name(self) -> &'static str {
        match self {
            Self::Iron => "Iron",
            Self::Copper => "Copper",
            Self::Silica => "Silica",
            Self::Carbon => "Carbon",
            Self::Crystal => "Crystal",
        }
    }
    pub fn glyph(self) -> char {
        match self {
            Self::Iron => 'i',
            Self::Copper => 'c',
            Self::Silica => 's',
            Self::Carbon => 'k',
            Self::Crystal => 'v',
        }
    }
    pub fn colour(self) -> u8 {
        match self {
            Self::Iron => 15,
            Self::Copper => 11,
            Self::Silica => 14,
            Self::Carbon => 10,
            Self::Crystal => 12,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Landmark {
    Ruin,
    Deposit(Resource),
    Arch,
    Crater,
}
impl Landmark {
    pub fn name(self) -> &'static str {
        match self {
            Self::Ruin => "Alien ruins",
            Self::Deposit(r) => r.name(),
            Self::Arch => "Basalt arch",
            Self::Crater => "Impact crater",
        }
    }
    pub fn glyph(self) -> char {
        match self {
            Self::Ruin => '#',
            Self::Deposit(r) => r.glyph(),
            Self::Arch => 'O',
            Self::Crater => '%',
        }
    }
    pub fn colour(self) -> u8 {
        match self {
            Self::Ruin => 13,
            Self::Deposit(r) => r.colour(),
            Self::Arch => 9,
            Self::Crater => 11,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    pub x: i64,
    pub y: i64,
    pub kind: Landmark,
}

/// Each footprint fits its region but can cross terrain chunk boundaries.
/// i128 intermediates handle partial regions at the i64 world edges.
pub fn at(seed: u64, x: i64, y: i64) -> Option<Site> {
    let gx = x.div_euclid(24);
    let gy = y.div_euclid(24);
    let h = hash(seed ^ 0x53495445, gx, gy);
    let kind = match h % 7 {
        0 => Landmark::Ruin,
        1 => Landmark::Deposit(Resource::Iron),
        2 => Landmark::Deposit(Resource::Copper),
        3 => Landmark::Deposit(Resource::Silica),
        4 => Landmark::Arch,
        5 => Landmark::Crater,
        _ => Landmark::Deposit(if h & 128 == 0 {
            Resource::Carbon
        } else {
            Resource::Crystal
        }),
    };
    let ax = i128::from(gx) * 24 + 2 + i128::from((h >> 8) % 20);
    let ay = i128::from(gy) * 24 + 2 + i128::from((h >> 16) % 20);
    let dx = i128::from(x) - ax;
    let dy = i128::from(y) - ay;
    let on_site = match kind {
        Landmark::Ruin => dx.abs().max(dy.abs()) == 2 && !(dx == 0 && dy == 2),
        Landmark::Deposit(_) => dx * dx + dy * dy <= 2,
        Landmark::Arch => dy == 0 && dx.abs() <= 1 || dx.abs() == 1 && dy == 1,
        Landmark::Crater => (3..=5).contains(&(dx * dx + dy * dy)),
    };
    if !on_site {
        return None;
    }
    Some(Site {
        x: ax.try_into().ok()?,
        y: ay.try_into().ok()?,
        kind,
    })
}
