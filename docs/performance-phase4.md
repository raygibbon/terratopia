# Phase 4 measured performance — 2026-10-23

Command: `cargo run --release --example ecosystem_benchmark`.
Five actual release runs are retained verbatim in [benchmark-phase4.json](benchmark-phase4.json).
Debian Linux x86_64, kernel 7.2.6, Intel Core i5-1340P (16 logical CPUs),
rustc 1.97.1 / LLVM 22.1.6, default release profile, one simulation thread.
No CPU affinity/governor control; the desktop and validation workloads were not
isolated. Variation is reported rather than treated as a speed guarantee.

## Workloads

A/B build exactly 100/1,000 distinct animals from real prototypes on passable
seed-827193 terrain. The fixtures begin hungry and mature, include all generated
categories, run real weather/feeding/hunting/navigation, and retain a spatial
index and persistent records. A uses 9 regions; B deliberately forces 130 regions
active, exceeding the normal radius-two gameplay window. This is a stress fixture,
not a claim that normal gameplay renders or continuously updates 1,000 animals.

Each runs 240 fixed ticks (60 seconds, up to 60 decisions per animal). Timing
includes the 180 cheap non-AI ticks, so both mean and p95/max are given. Corpses
remain in the 100/1,000 records; final live populations are 98/971. A performs
5,932 decisions/queries and 12 A* expansions; B 58,946 and 521. No event overflow.

C begins with exactly 10,000 **represented** dormant animals in 500 population
summaries of 20 animals each. It runs 24,000 ticks, dispatching 50,000 region
updates, **zero individual AI decisions and zero individual creature records**.
Population totals can change under the coarse model. This measures aggregate
representation, not saving 10,000 fully materialized creatures or 10,000 active AI.

## Results

All times below are milliseconds. Medians across five runs; ranges are min–max.

| Measurement | A: 100 | B: 1,000 |
| --- | ---: | ---: |
| Total 240 ticks | 17.101 (16.384–20.392) | 550.548 (521.867–622.784) |
| Mean per fixed tick | 0.0713 (0.0683–0.0850) | 2.2940 (2.1744–2.5949) |
| Per-run p95 tick | 0.3355 (0.3219–0.4056) | 9.9489 (9.6293–11.4887) |
| Per-run maximum tick | 0.8132 (0.6731–0.8571) | 34.8314 (32.9532–35.1783) |
| Index reconciliation, total | 1.564 | 21.119 |
| AI, inclusive total | 15.493 | 529.251 |
| Nearby queries within AI | 6.460 | 94.042 |
| Local A* within AI | 0.030 | 2.236 |
| Separate 10,000 nearby queries | 7.398 (7.383–8.377) | 17.423 (17.159–18.076) |
| 300 logical 100×30 redraws | 78.910 (76.969–102.890) | 86.731 (84.826–114.840) |

AI timing includes queries, pathfinding, terrain/flora/weather, needs, combat,
cloning a small species record and persistence bookkeeping. Do not add its
submeasurements together as independent totals. Instrumentation uses Instant;
these times/counters are observational and do not drive AI or enter saves.

C's 24,000 ticks: **1.465 ms median (1.425–1.890 ms)**. The retained summary key
and value payloads total 20,000 bytes, excluding BTreeMap node/allocator overhead.

| Storage observation | A | B |
| --- | ---: | ---: |
| Generational entity arena Vec capacity | 3,072 B | 24,576 B |
| Dense creature Vec capacity | 16,384 B | 131,072 B |
| Remembered state payloads | 5,600 B | 56,000 B |

These are component estimates, excluding spatial/tree nodes, String allocations,
Vec headers, allocator and terminal buffers. Linux process peak RSS across the
whole executable was **3,876–3,996 KiB**, including code/libraries, fixtures and
rendering. It is not an isolated measurement of the dormant summaries.

## Findings and boundaries

The recorded ticks stay below the game's 250 ms fixed step, including stress
maxima, on this host. This is not a promise about slow terminals or other CPUs.
Only visible creatures render; logical redraw measurements exclude terminal
writes, emulator rasterisation, SSH throughput and input-to-display latency.
Actual terminal behaviour is covered separately by PTY tests and xterm captures.

The stress fixture exceeds the retained 128 terrain chunks and causes **5,820
terrain-cache misses/evictions** during 60 seconds, versus A's 4 misses and zero
evictions. AI dominates, with terrain/flora accesses and persistence work among
its remaining costs; the coarse counters identify cache pressure rather than
proving a precise percentage for each operation. Pathfinding is small in this
workload after approaching occupied targets via free adjacent cells. No result
justifies replacing the ECS/storage, adding threads or resizing caches globally.

Next profiling should isolate forage tile sampling and cache generation under
controlled CPU conditions, then test a larger active-window setting before
changing the cache policy. Activation currently scans bounded remembered
records; region-local record indexing may help long-travel workloads. A/B's
short throughput runs and the separate 30-minute stability test do not establish
permanent balance on every hostile world. macOS/Windows performance is unmeasured.

Pre-change workloads and review context are recorded in [ecosystem.md](ecosystem.md).
They had only 13 active animals, so there is no direct before/after speedup claim.
