pub mod biome;
pub mod chunk;
pub mod flora;
pub mod landmark;
pub mod noise;
pub mod profile;
pub mod seed;
pub mod terrain;
use chunk::{Chunk, ChunkKey, locate};
use terragame_world::ChunkCache;
use terrain::Tile;

pub use terragame_world::CacheStats;

/// Distance-bounded cache; generation never observes cache contents.
pub struct World {
    seed: u64,
    profile: profile::Profile,
    chunks: ChunkCache<Chunk>,
}
impl World {
    pub fn new(seed: u64, capacity: usize) -> Self {
        Self::with_profile(seed, capacity, profile::Profile::generate(seed))
    }
    pub fn legacy(seed: u64, capacity: usize) -> Self {
        Self::with_profile(seed, capacity, profile::Profile::legacy())
    }
    fn with_profile(seed: u64, capacity: usize, profile: profile::Profile) -> Self {
        Self {
            seed,
            profile,
            chunks: ChunkCache::new(capacity),
        }
    }
    pub fn profile(&self) -> profile::Profile {
        self.profile
    }
    pub fn generator(&self) -> u8 {
        if self.profile.legacy { 2 } else { 3 }
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    /// Find a landing site with walkable terrain beyond the safety clearing.
    /// The bounded search uses generation directly, leaving the cache cold.
    pub fn landing_site(&self) -> (i64, i64) {
        let safe = |x, y| {
            [(0, 0), (8, 0), (-8, 0), (0, 8), (0, -8)]
                .iter()
                .all(|&(dx, dy)| {
                    terrain::generate_with(self.seed, x + dx, y + dy, &self.profile)
                        .biome
                        .passable()
                })
        };
        for radius in 0i64..=64 {
            for y in -radius..=radius {
                for x in -radius..=radius {
                    if x.abs().max(y.abs()) == radius && safe(x * 16, y * 16) {
                        return (x * 16, y * 16);
                    }
                }
            }
        }
        (0, 0) // The deterministic clearing is always safe, even in this fallback.
    }
    pub fn focus(&mut self, x: i64, y: i64) {
        self.chunks.focus(locate(x, y).0);
    }
    pub fn cached_chunks(&self) -> usize {
        self.chunks.len()
    }
    pub fn contains_chunk(&self, key: ChunkKey) -> bool {
        self.chunks.contains(key)
    }
    pub fn tile(&mut self, x: i64, y: i64) -> Tile {
        let (key, index) = locate(x, y);
        let seed = self.seed;
        let profile = self.profile;
        self.chunks
            .get_or_insert_with(key, |key| Chunk::generate(seed, key, &profile))
            .tile(index)
    }
    pub fn cache_stats(&self) -> CacheStats {
        self.chunks.stats()
    }
}
