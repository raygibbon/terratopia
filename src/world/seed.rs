// Shared engine hashing preserves the original SplitMix64 coordinate stream.
pub use terragame_core::random::{coordinate_hash as hash, mix};

pub fn planet_name(seed: u64) -> String {
    let names = [
        "Aster", "Nacre", "Vesper", "Thalos", "Iona", "Orin", "Sable", "Nyx",
    ];
    format!(
        "{}-{:03}",
        names[(mix(seed) % 8) as usize],
        mix(seed ^ 71) % 1000
    )
}
