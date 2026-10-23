# Phase 2 validation

Date: 23 October 2026. Linux x86_64, stable rustc/cargo 1.97.1. Terrabox remains
unchanged. Existing terrain/noise, chunk/cache, exploration and terminal
architecture are preserved. No runtime/test dependency was added.

| Check | Actual result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | Pass: 36 tests (22 existing, 14 survival tests) |
| `cargo build --release` | Pass |
| `cargo check --target x86_64-pc-windows-gnu` | Pass, compilation only |
| `python3 scripts/smoke-terminal.py --full-night` | Pass, including actual timed nightfall/dawn |
| `python3 scripts/smoke-terminal.py` | Pass, repeated for final UI and save-error recovery |
| `python3 scripts/capture-terminal.py` | Actual xterm crafting/survival captures generated |
| `git diff --check` | Pass |

## Rust coverage

The existing 22 tests retain biome/noise determinism, full chunk/reference
comparisons, negative/boundary/extreme coordinates, cache bounds, collision,
camera/resize, fog, scanner, state and fixed-step checks.

The 14 new tests cover:

- Complete crash -> cache -> iron/carbon -> mining tool -> crystal -> shelter
  kit -> construction -> powered oxygen -> full night -> dawn success, using
  gameplay actions and all 480 simulation ticks.
- Safe reachable crash clearings and starter nodes for five seeds.
- Stack splitting/merging, capacity and atomic failed crafting, harvesting and
  scavenging; every recipe's exact inputs/output.
- Tool gates, one-time cache collection, partial/full depletion and regeneration
  independence even after cache eviction.
- Oxygen drain/suffocation, canisters, powered refill, temperature, night,
  thermal equipment, radiation and shelter protection.
- Frozen simulation in inventory/crafting/help/pause, death stopping time, and
  the requirement for a complete uninterrupted sheltered night.
- Save/load roundtrip of player, seed, vitals, items, power, structures, harvested
  cells, fog, discovery counts and partial night progress; continuation to dawn.
- Actual file replacement; an occupied temporary path preserves the prior save.
- Rejection of unsupported versions, malformed/out-of-range, truncated,
  oversized and trailing save data. Procedural carbon/crystal occurrence.

## Actual release-binary terminal tests

The Unix PTY script uses isolated temporary save paths, not the player's default
save. Each of three runs lands, scavenges, scans, checks repeat-scan deduplication,
opens inventory/crafting/help, checks missing-ingredient feedback, gathers iron
and carbon, crafts a tool, gathers crystal, crafts/builds shelter, powers its
station, saves, moves away, loads and verifies the loaded game is paused. It
resizes to 20x6 and back to 100x30 in both inventory and pause states.

The `--full-night` Q run then **waited in real time**, without clock manipulation,
through nightfall and dawn until the actual terminal output reported
`OBJECTIVE COMPLETE`. This is separate from the fast 480-tick Rust progression
test. The shelter had been built and powered through ordinary terminal input.
Q, Escape and Ctrl+C were tested independently: exit status, exact termios
restoration, alternate-screen exit and cursor restoration passed.

Menu, inventory, help and paused states showed no unsolicited output during
600ms idle samples; measured Linux CPU ticks were zero in those samples.
Active play now intentionally redraws time/vitals once per second and is no
longer subject to the old Phase 1.1 unchanged-playing-output assertion.

A separate PTY check selects an unwritable save location, verifies Q reports the
error and leaves the game open, and verifies Ctrl+Q exits with terminal attributes
restored. Starting with a missing `--load` file fails cleanly before entering raw
mode. The save-error test and existing save-file tests exercise different failure
paths; neither claims power-failure durability.

## Visual inspection and limits

The actual xterm release game was captured at 100x30 using DejaVu Sans Mono 12pt.
`crafting.png` shows the selected shelter recipe and owned/required ingredients.
`survival.png` shows the constructed shelter under the player, nearby ship and
starter nodes, live vitals, station fuel, objective and scanner. Images were
inspected for readability, correct gameplay state and personal information.
See [capture instructions](../assets/screenshots/README.md).

No physical keyboard session, emulator flicker/latency benchmark or native
Windows, macOS, SSH, Android/Termux runtime test was performed. Xterm capture
uses synthetic X key events; PTY tests use terminal bytes and resize signals.
The Windows result is a cross-check, not native console execution. Forced
termination/abort recovery is not claimed.

Remaining scope limits: single-cell shelters, automatic equipment while carried,
no tool wear, no food/thirst, energy is advisory, copper/silica have no recipes,
non-occluded sensor sight, no save migration/cloud sync, finite save limits,
unbounded in-memory exploration history and synchronous generation. The Phase
1.1 performance report is historical; no new Phase 2 throughput benchmark is
claimed. Next work should focus on balance/playtesting and native platform
validation before adding broader simulation or travel.
