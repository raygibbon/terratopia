# Phase 4 validation — 2026-10-23

All implementation changes remain uncommitted for review. No Git history was
rewritten. TerraTopia and TerraGame changed; Terrabox remained unchanged.

## Automated checks

Executed successfully in both TerraTopia and TerraGame:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

TerraTopia: **69 tests**, including the original 53 and 16 ecosystem tests.
TerraGame: **15 tests**, including its original 14 and bounded navigation coverage.
Also ran `cargo build --release --examples`, a release unassisted ecological
stability test, the existing pre-change benchmarks, and the five release runs
of `ecosystem_benchmark` recorded in benchmark-phase4.json.

New coverage includes persistent identity vs stale arena handles; creation,
removal and indexed position queries; deterministic AI; predator attack/death/eat
FIFO ordering; scavenging/one-time remains; shared plant depletion; hunger and
rest; population births/caps; corpse persistence; dormant summary-only work;
activation order/cache order; eviction/reload; version-1 and genuine version-2
migration/backups; uninterrupted vs save/resumed AI; reference/resource limits;
weather/death; read-only resize; queue backpressure; bounded local A* with an
occupied target and new shelter, wall changes, exhaustion and extreme coordinates.

An accelerated controlled demographic test runs 1,200 simulated seconds with
fed hardy animals to isolate reproduction/identity/cap rules. A separate
**unassisted 1,800-second (30-minute)** starter ecosystem runs all normal feeding,
hunting and weather rules: **62 initial active animals, 78 births, 115 alive and 308 retained
records at the end**. These tests cover different failure modes; neither is a
multi-seed, unlimited-time balance proof.

## Real terminals

`python3 scripts/smoke-terminal.py --full-night` exercised the actual TerraTopia
release executable in Unix PTYs. Passed crash/cache/gather/craft/build, scans,
menus/pause/resize, save/load, q/Esc/Ctrl+C, failed save and Ctrl+Q restoration,
toxic-planet wildlife/flora/crafting/ruin excursion, hostile escape, real forecast
and storm, powered shelter, real-time night/dawn objective, persisted completion
and resumed load. Short 600 ms menu/paused windows measured zero CPU ticks.

`python3 scripts/smoke-terminal.py` in TerraGame passed TerraSnake and sandbox
movement, scoring/gathering, multirate/region behaviour, pause/resize/restart,
q/Esc/Ctrl+C and terminal restoration. Short paused samples reported zero CPU ticks.

`python3 scripts/capture-ecosystem.py` captured actual X11 xterm gameplay and the
M inspection panel. Both PNGs were visually inspected. The helper sends events
only to its own game window and preserves desktop focus. Images are in
assets/screenshots/ecosystem.png and ecosystem-inspection.png.

## Review findings and remaining work

Review corrected: original slot-0–2 species/category meanings; starting far from
camp must not pre-create starter animals without their summary; move/leash margin
vs save validation; old schemas with kills across many regions; same-home birth
conflicts; resting hysteresis; occupied A* goals; stale occupancy on kills;
explicit event overflow; genuine old-save backups, player/birth occupancy, scavenger hostility flags,
and deferring birth-ID allocation until mature parents exist. Existing gameplay, climate,
inventory/crafting and generator regression tests remain intact.

Limits are documented in ecosystem.md: 10,000 retained records/summary ceiling,
finite non-reused birth candidates, frozen dormant individual needs, approximate
coarse births/mortality, permanent harvested flora, local path budgets and no
long-distance migration. The event queue reports committed facts; current rules
resolve transactions directly rather than requiring asynchronous event replay.

Next steps: profile stress terrain/flora/cache access; balance multiple seeds
and longer climates without raising limits speculatively; then consider regional
record indexing and a versioned wider birth-ID namespace if actual play reaches
those ceilings. Native **Windows and macOS runtime/PTY/performance remain
unverified**. No cross-platform or universal filesystem durability claim is made.

## Changed files

TerraTopia: `src/wildlife.rs` (ecosystem and engine integration),
`src/persistence.rs` (schema 3/migration), `src/game.rs`, `src/renderer.rs`,
`src/ui/mod.rs` (tick integration and presentation); `tests/ecosystem.rs`,
`tests/living.rs`, `tests/survival.rs`, `tests/fixtures/phase3.save` and its README;
`examples/ecosystem_benchmark.rs` and adapted `examples/benchmark.rs`;
`scripts/smoke-terminal.py`, `scripts/capture-ecosystem.py` and reusable capture
helper; README, integration/review reports, ecosystem/performance/validation
reports, raw benchmark JSON, screenshot guide and two real gameplay PNGs.

TerraGame: `crates/terragame-world/src/navigation.rs`, its export in `lib.rs`,
`tests/engine.rs`, README and architecture/integration/validation documentation.
Terrabox: no changed files.
