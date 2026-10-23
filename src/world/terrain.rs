use super::{
    biome::Biome,
    landmark::{self, Landmark},
    noise::field,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    pub biome: Biome,
    pub landmark: Option<Landmark>,
}
pub fn generate(seed: u64, x: i64, y: i64) -> Tile {
    generate_with(seed, x, y, &super::profile::Profile::generate(seed))
}
pub fn generate_legacy(seed: u64, x: i64, y: i64) -> Tile {
    generate_with(seed, x, y, &super::profile::Profile::legacy())
}
pub(crate) fn generate_with(seed: u64, x: i64, y: i64, profile: &super::profile::Profile) -> Tile {
    from_climate(
        seed,
        x,
        y,
        (
            field(seed, x, y),
            field(seed ^ 0xa172, x, y),
            field(seed ^ 0xb891, x, y),
        ),
        profile,
    )
}
pub(crate) fn from_climate(
    seed: u64,
    x: i64,
    y: i64,
    climate: (i64, i64, i64),
    profile: &super::profile::Profile,
) -> Tile {
    let (elevation, temperature, moisture) = profile.climate(climate.0, climate.1, climate.2);
    // A deterministic landing clearing guarantees a walkable start for every seed.
    let biome = if (-2..=2).contains(&x) && (-2..=2).contains(&y) {
        Biome::Grassland
    } else {
        Biome::classify(elevation, temperature, moisture)
    };
    let landmark = biome
        .passable()
        .then(|| landmark::at(seed, x, y))
        .flatten()
        .map(|site| site.kind);
    Tile { biome, landmark }
}
