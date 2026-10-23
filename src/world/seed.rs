/// Stable SplitMix64 finalizer: intentional wrapping is confined to hashing.
pub fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

pub fn hash(seed: u64, x: i64, y: i64) -> u64 {
    mix(seed ^ mix(x as u64) ^ mix((y as u64).wrapping_add(0x9e3779b97f4a7c15)))
}

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
