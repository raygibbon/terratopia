# Phase 3 performance measurements

Measured on Linux x86_64 with Rust 1.97.1, release mode, seed 827193 and generator
3 on 2026-10-23. Five consecutive executions of the release benchmark are stored
in [benchmark-phase3.json](benchmark-phase3.json). Shared desktop load varies;
these are workload measurements, not portable latency guarantees or a claim of
speedup over earlier phases.

```sh
cargo run --release --example benchmark
# Subsequent samples use the same compiled executable:
target/release/examples/benchmark
```

| Workload | Median total (ms) | Range (ms) |
|---|---:|---:|
| 500 cold chunks | 43.452 | 41.472–47.017 |
| 1,000,000 cached tile lookups | 11.467 | 9.963–12.033 |
| Travel through 500 chunks | 41.804 | 39.117–48.048 |
| 300 scrolling logical frames, 100×30 | 138.005 | 136.215–146.282 |
| 500 full scan actions | 50.475 | 47.702–53.776 |
| 1,000 one-second creature updates | 5.291 | 4.637–5.480 |
| 1,000 legal player movements | 34.917 | 33.698–35.576 |

The scrolling workload advances the viewport geometrically through terrain,
updates observation and active wildlife, and renders into a Terrabox Surface.
It deliberately crosses blocked tiles to measure a moving viewport. The separate
movement workload uses actual Game actions on reachable crash-site ground,
resetting energy between steps to isolate input processing. Full scans use the
actual Game scan action at a populated crash site, including discovery messages,
flora and species. Creature updates start with 13 active animals; all 13 remain
alive at the end. This exercises movement and interactions, without repeated
player-death pauses.

Rendering measurements exclude terminal writes and emulator rasterization. They
do not establish SSH throughput, flicker or physical keyboard latency. At this
viewport the measured median logical frame is about 0.460 ms, a full scan about
0.101 ms and a one-second creature update about 0.005 ms. No measured workload
justified extra worker threads or dependencies.

Cache travel retained exactly 128 chunks, with 1,000,001 hits, 500 misses and
372 evictions (counters include the preceding cached lookup workload). Wildlife
is limited to 64 active entities within radius 48, with a bounded persistent
injury/death ledger; weather is a pure query without a growing regional cache.
The fixed loop caps catch-up at four ticks. Existing surface diffing avoids
unchanged terminal output, and inactive screens block for input. PTY samples
of menus, inventory, help and pause showed zero process CPU ticks and no output
in each 600 ms idle window. That short observation is not a long-duration power
measurement.

The important performance choices here are bounded simulation, local terrain
queries, reuse of existing chunk caching and sleeping modal screens. No
comparative profiling demonstrated a generation or rendering bottleneck that
required changing the established engine architecture.
