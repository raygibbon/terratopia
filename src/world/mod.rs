pub mod biome;
pub mod chunk;
pub mod landmark;
pub mod noise;
pub mod seed;
pub mod terrain;
use chunk::{Chunk, ChunkKey, locate};
use std::collections::BTreeMap;
use terrain::Tile;

#[derive(Clone, Copy, Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

/// Distance-bounded cache; generation never observes cache contents.
pub struct World {
    seed: u64,
    capacity: usize,
    focus: ChunkKey,
    chunks: BTreeMap<ChunkKey, Chunk>,
    stats: CacheStats,
}
impl World {
    pub fn new(seed: u64, capacity: usize) -> Self {
        Self {
            seed,
            capacity: capacity.max(1),
            focus: (0, 0),
            chunks: BTreeMap::new(),
            stats: CacheStats::default(),
        }
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
                    terrain::generate(self.seed, x + dx, y + dy)
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
        self.focus = locate(x, y).0;
    }
    pub fn cached_chunks(&self) -> usize {
        self.chunks.len()
    }
    pub fn contains_chunk(&self, key: ChunkKey) -> bool {
        self.chunks.contains_key(&key)
    }
    pub fn tile(&mut self, x: i64, y: i64) -> Tile {
        let (key, index) = locate(x, y);
        if let Some(chunk) = self.chunks.get(&key) {
            self.stats.hits = self.stats.hits.saturating_add(1);
            return chunk.tile(index);
        }
        self.stats.misses = self.stats.misses.saturating_add(1);
        if self.chunks.len() >= self.capacity {
            let distance = |k: &ChunkKey| {
                (i128::from(k.0) - i128::from(self.focus.0))
                    .abs()
                    .max((i128::from(k.1) - i128::from(self.focus.1)).abs())
            };
            if let Some(farthest) = self.chunks.keys().max_by_key(|k| distance(k)).copied() {
                self.chunks.remove(&farthest);
                self.stats.evictions = self.stats.evictions.saturating_add(1);
            }
        }
        let chunk = Chunk::generate(self.seed, key);
        let tile = chunk.tile(index);
        self.chunks.insert(key, chunk);
        tile
    }
    pub fn cache_stats(&self) -> CacheStats {
        self.stats
    }
}
