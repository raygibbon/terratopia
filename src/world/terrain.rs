use super::{biome::Biome, noise::field};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile {
    pub biome: Biome,
}
pub fn generate(seed: u64, x: i64, y: i64) -> Tile {
    // A deterministic landing clearing guarantees a walkable start for every seed.
    let biome = if (-2..=2).contains(&x) && (-2..=2).contains(&y) {
        Biome::Grassland
    } else {
        Biome::classify(
            field(seed, x, y),
            field(seed ^ 0xa172, x, y),
            field(seed ^ 0xb891, x, y),
        )
    };
    Tile { biome }
}
