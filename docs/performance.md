# Phase 1.1 performance

Measured on Linux x86_64, stable rustc 1.97.1, release profile, seed 827193,
23 October 2026. No dependency, worker thread or engine change was added.

```sh
cargo run --release --example benchmark
```

The example benchmarks CPU work only. It excludes terminal `present`, terminal
output transport, emulator rasterization and physical display latency. Scrolling
uses a fixed geometric route (including across impassable terrain) to isolate
visibility updates and logical frame construction rather than route finding.
It reuses a 100x30 Surface and includes normal sight/discovery updates each step.

## Comparison

The first Phase 1 baseline sample took 200.59ms for 500 cold chunks and 101.35ms
for 300 full-map logical redraws. That baseline has no fog or landmarks, so it
is context rather than a direct feature-equivalent comparison.

The primary comparison below uses Phase 1.1 landmarks/fog/scanner code both
before and after optimization. Five samples were collected for each. The
unoptimized benchmark binary was retained before applying chunk lattice reuse.
The final version also adds cache counters, a single-lookup cache hit path and
minor palette/readability changes. Medians are shown; raw samples are in
[benchmark-results.json](benchmark-results.json).

| Workload | Before median | Final median | Final range |
| --- | ---: | ---: | ---: |
| Generate 500 cold chunks | 149.06ms | 35.41ms | 31.84–57.51ms |
| 1,000,000 warm tile lookups | 8.79ms | 12.26ms | 9.37–28.48ms |
| Travel through 500 chunk positions | 147.08ms | 35.58ms | 27.72–76.41ms |
| 300 scrolling logical redraws at 100x30 | 96.38ms | 104.00ms | 99.52–126.01ms |

The demonstrated bottleneck was redundant coherent-noise work during chunk
generation. Every aligned 32x32 chunk shares four corner hashes per octave at
scales 32/64/128. Reusing these values and interpolation weights rather than
recomputing them per tile reduces cold generation time by about **76% (4.2x)**
in this comparison. Fixed-point interpolation and rounding remain exact; tests
compare every tile in selected chunks against the unchanged coordinate sampler,
including negative and extreme chunks. Cache traversal benefits from the same
change, reducing its median by about 76%.

Warm lookup and scrolling medians increased in this run. There is **no measured
scrolling speedup** claimed. Cache statistics add work even on hits; timings also
show substantial host scheduling variation. The final scrolling result is about
0.35ms per logical frame, before terminal I/O. Further row caching or worker
threads would add complexity without a demonstrated need in these workloads.
The numbers are small smoke benchmarks, not statistically controlled guarantees.

## Cache behaviour

The travel workload uses the existing distance-based 128-chunk bound. Counters
including its setup and warm-lookup workload consistently report:

```text
hits: 1,000,001
misses: 500
evictions: 372
retained chunks: 128
```

There is no whole-world generation. Fog memory is independent of this cache:
unknown screen cells require no terrain lookup; revisiting evicted explored
terrain regenerates the same landmarks without losing exploration masks or
site records. The warm path uses one BTreeMap lookup instead of two, but the
instrumented benchmark does not demonstrate a resulting speed gain.

The PTY smoke test measured zero CPU ticks and no output in nine 600ms menu,
playing-idle and paused samples. Playing sleeps between four simulation wakes
per second; menu/pause have no deadline. These samples do not constitute a
long-duration power/CPU benchmark.

Remaining limits: synchronous generation, unbounded expedition knowledge,
128-chunk terrain capacity, and potential thrashing for very large remembered
viewports. No emulator output-throughput benchmark or cross-platform runtime
performance claim is made.
