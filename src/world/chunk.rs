use super::{
    noise::chunk_field,
    terrain::{Tile, from_climate},
};
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
        let bx = key.0 * CHUNK_SIZE;
        let by = key.1 * CHUNK_SIZE;
        let elevation = chunk_field(seed, bx, by);
        let temperature = chunk_field(seed ^ 0xa172, bx, by);
        let moisture = chunk_field(seed ^ 0xb891, bx, by);
        let mut tiles = Vec::with_capacity(1024);
        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let i = (y * CHUNK_SIZE + x) as usize;
                tiles.push(from_climate(
                    seed,
                    bx + x,
                    by + y,
                    elevation[i],
                    temperature[i],
                    moisture[i],
                ));
            }
        }
        Self { tiles }
    }
    pub(crate) fn tile(&self, index: usize) -> Tile {
        self.tiles[index]
    }
}
