use super::terrain::{Tile, generate};
pub const CHUNK_SIZE: i64 = 32;
pub type ChunkKey = (i64, i64);
pub fn locate(x: i64, y: i64) -> (ChunkKey, usize) {
    (
        (x.div_euclid(CHUNK_SIZE), y.div_euclid(CHUNK_SIZE)),
        (y.rem_euclid(CHUNK_SIZE) * CHUNK_SIZE + x.rem_euclid(CHUNK_SIZE)) as usize,
    )
}
pub(crate) struct Chunk {
    tiles: Vec<Tile>,
}
impl Chunk {
    pub(crate) fn generate(seed: u64, key: ChunkKey) -> Self {
        let mut tiles = Vec::with_capacity(1024);
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                tiles.push(generate(
                    seed,
                    key.0 * CHUNK_SIZE + x,
                    key.1 * CHUNK_SIZE + y,
                ));
            }
        }
        Self { tiles }
    }
    pub(crate) fn tile(&self, index: usize) -> Tile {
        self.tiles[index]
    }
}
