use super::seed::hash;
const UNIT: i64 = 65536;
fn smooth(t: i64) -> i64 {
    t * t / UNIT * (3 * UNIT - 2 * t) / UNIT
}
fn lerp(a: i64, b: i64, t: i64) -> i64 {
    a + (b - a) * t / UNIT
}

/// Fixed-point value noise. Divide before interpolation to retain local detail
/// even near i64 limits; no floating-point platform differences or seams.
fn value(seed: u64, x: i64, y: i64, scale: i64) -> i64 {
    let gx = x.div_euclid(scale);
    let gy = y.div_euclid(scale);
    let tx = smooth(x.rem_euclid(scale) * UNIT / scale);
    let ty = smooth(y.rem_euclid(scale) * UNIT / scale);
    let sample = |dx, dy| (hash(seed, gx + dx, gy + dy) & 65535) as i64;
    lerp(
        lerp(sample(0, 0), sample(1, 0), tx),
        lerp(sample(0, 1), sample(1, 1), tx),
        ty,
    )
}

pub fn field(seed: u64, x: i64, y: i64) -> i64 {
    (value(seed, x, y, 128) * 4 + value(seed ^ 19, x, y, 64) * 2 + value(seed ^ 37, x, y, 32)) / 7
}

/// All scales divide an aligned chunk: its tiles share four lattice corners
/// per octave. Reuse these hashes and interpolation weights without changing
/// fixed-point rounding or the public coordinate sampler's output.
pub(crate) fn chunk_field(seed: u64, x: i64, y: i64) -> [i64; 1024] {
    let mut output = [0; 1024];
    for (salt, scale, weight) in [(0, 128, 4), (19, 64, 2), (37, 32, 1)] {
        let gx = x.div_euclid(scale);
        let gy = y.div_euclid(scale);
        let sample = |dx, dy| (hash(seed ^ salt, gx + dx, gy + dy) & 65535) as i64;
        let (a, b, c, d) = (sample(0, 0), sample(1, 0), sample(0, 1), sample(1, 1));
        let top: [i64; 32] = std::array::from_fn(|i| {
            lerp(
                a,
                b,
                smooth((x + i as i64).rem_euclid(scale) * UNIT / scale),
            )
        });
        let bottom: [i64; 32] = std::array::from_fn(|i| {
            lerp(
                c,
                d,
                smooth((x + i as i64).rem_euclid(scale) * UNIT / scale),
            )
        });
        for row in 0..32 {
            let ty = smooth((y + row as i64).rem_euclid(scale) * UNIT / scale);
            for col in 0..32 {
                output[row * 32 + col] += lerp(top[col], bottom[col], ty) * weight;
            }
        }
    }
    for value in &mut output {
        *value /= 7;
    }
    output
}
