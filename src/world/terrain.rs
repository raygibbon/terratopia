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
    from_climate(
        seed,
        x,
        y,
        field(seed, x, y),
        field(seed ^ 0xa172, x, y),
        field(seed ^ 0xb891, x, y),
    )
}
pub(crate) fn from_climate(
    seed: u64,
    x: i64,
    y: i64,
    elevation: i64,
    temperature: i64,
    moisture: i64,
) -> Tile {
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
