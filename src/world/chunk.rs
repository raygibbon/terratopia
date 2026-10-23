use super::{
    noise::chunk_field,
    terrain::{Tile, from_climate},
};
pub const CHUNK_SIZE: i64 = 32;
pub type ChunkKey = (i64, i64);
pub fn locate(x: i64, y: i64) -> (ChunkKey, usize) {
    terragame_world::ChunkLayout::new(CHUNK_SIZE as u16)
        .expect("nonzero chunk side")
        .locate((x, y).into())
}
pub(crate) struct Chunk {
    tiles: Vec<Tile>,
}
impl Chunk {
    pub(crate) fn generate(seed: u64, key: ChunkKey, profile: &super::profile::Profile) -> Self {
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
                    (elevation[i], temperature[i], moisture[i]),
                    profile,
                ));
            }
        }
        Self { tiles }
    }
    pub(crate) fn tile(&self, index: usize) -> Tile {
        self.tiles[index]
    }
}
