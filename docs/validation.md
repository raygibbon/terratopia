# Phase 3 validation — Living Alien Worlds

23 October 2026; Linux x86_64, stable Rust 1.97.1. Terrabox remains unchanged;
Third-party dependencies and edition 2024 are unchanged. Three local TerraGame
crates now provide shared infrastructure. Existing source/history and
the Phase 2 survival loop were preserved.

| Check | Actual result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | Pass: 53 tests, including 17 living-world tests |
| `cargo build --release` | Pass |
| `cargo check --target x86_64-pc-windows-gnu` | Pass; compilation only |
| `python3 scripts/smoke-terminal.py` | Pass: release-binary terminal scenarios |
| `python3 scripts/smoke-terminal.py --full-night` | Pass: real timed exploration/storm/night/dawn scenario |
| `cargo run --release --example benchmark` | Executed, with five-sample CPU workloads recorded |
| `python3 scripts/capture-terminal.py` | Actual xterm gameplay, crafting, survey and toxic-storm captures |
| `git diff --check` | Pass |

## Automated behaviour

All 36 previous tests remain. The 17 living-world tests add:

- Deterministic profiles, occurrence of seven archetypes, actual terrain/yield
  differences and exact new/legacy chunk/reference agreement at negative and
  extreme coordinates.
- Completion of the starter survival loop on every planet archetype.
- Deterministic species/population, bounded activation, eviction independence,
  grazer flight, territorial defence, predator chase/disengagement, obstruction,
  occupancy/player collision and shelter protection.
- Weather warning -> onset -> severity -> clearing, reduced sensor radii and
  repeatability, plus combined climate/oxygen/weather/equipment exposure.
- Finite flora yields/depletion across eviction/save/load, atomic inventory
  interactions, deterministic one-time ruin salvage and persistent kills/corpses.
- Energy exhaustion/rest recovery, restorative consumables, modal freezing of
  all simulation, resize, bounded catch-up and immediate lethal plant contact.
- A harvested plant with a constructed shelter roundtrips correctly.
- Genuine pre-change v1 import, exact legacy terrain/progress, continuation to
  dawn, v2 rewrite and exact CRLF backup preservation; v2 active state/effects,
  weather reconstruction, malformed extensions and future-version rejection.

The fixture was captured with the original Phase 2 encoder, not constructed by
changing a schema-2 header. Provenance is in tests/fixtures/README.md. Existing
file tests also check truncation, size/capacity limits and temporary-file failure
without changing the previous save.

## Actual terminal acceptance

The standard PTY suite launches actual release binaries, using isolated temporary
save paths. Q, Escape and Ctrl+C runs perform the original crash/scavenge/gather/
craft/build/power loop, scanning, inventory, recipe failure, help, pause, saving,
loading and resizing to 20x6/back to 100x30. Exit status, exact termios restoration,
alternate-screen exit and cursor restoration are asserted. Menu/inventory/help/
pause samples show no unsolicited output and measured zero CPU ticks over 600ms.
Active play intentionally wakes four times and redraws about once per second.

The living scenario continues the **same toxic-world expedition** (seed 2) with
ordinary keyboard input. It scans wildlife, opens the planetary survey, rests,
mines extra carbon, harvests starleaf/fungus, crafts a medical kit, reaches the
ruin, salvages it, confirms no second loot, encounters a hostile animal and
escapes back to the constructed powered shelter. It observes a real forecast
warning and actual toxic-storm onset, saves the plant/ruin state, loads and
continues exploring.

With `--full-night`, that same expedition waits through actual active simulation
until `OBJECTIVE COMPLETE`, saves, asserts the stored completion flag, loads and
continues. There is **no edited clock or fast-forward fixture** in this PTY
scenario. The Rust tests separately advance fixed ticks to check night rules
quickly. A bounded scripted route avoids occupied cells using only known crash/
trail geometry; it does not bypass the game input or collision system.

A separate PTY case verifies an unwritable save path keeps the live game open,
Ctrl+Q exits without saving and restores terminal state, and missing startup
loads fail before entering raw mode.

## Screenshots, performance and limits

Actual Linux xterm images were inspected: initial wildlife/vegetation, a paused
survey with functional profile data, crafting, and a real toxic storm after a
40-second running interval. They are window captures of actual game output,
not generated art or reconstructed terminal text. Seed 2 is a toxic jungle;
100x30, DejaVu Sans Mono 12pt. See assets/screenshots/README.md.

CPU benchmarks cover cold chunks, warm cache access/travel, geometric scrolling,
full scanner actions, 13 active creatures and normal valid movement. Raw samples
and methodology are in performance-phase3.md / benchmark-phase3.json. No emulator
throughput, physical keyboard latency or statistical frame-time guarantee is
claimed. No background workers were justified or added.

No native Windows/macOS/Termux/SSH runtime or physical keyboard/flicker sign-off
was performed. Windows was cross-checked only. The screenshot script sends
synthetic X events to its own terminal; PTY tests send actual terminal bytes and
resize signals. Forced termination/abort recovery remains outside RAII guarantees.

Remaining limits: local greedy AI can get stuck, untouched ambient animals can
respawn, permanent plant/mineral depletion, single-cell shelters, finite save
caps, growing expedition history, transient last-scan/events not saved, and no
weather animation, galaxy, travel or multiplayer. Recommended next milestone:
balance this loop with playtesting, improve navigation where measured/observed,
and validate the remaining native terminal targets.

## TerraGame extraction validation

After adding the engine path dependencies and migrating fixed steps, camera,
hashing, chunk indexing/cache and terminal Session, all 53 game tests pass.
Formatting, Clippy and the release build pass. The actual post-integration
`python3 scripts/smoke-terminal.py --full-night` run passed the original three
exit/resize/pause/gather/craft/build/save/load cases plus the connected toxic-world
expedition through real dawn. The script saved the completion flag, loaded it
and continued. No clock or fixture fast-forward was used. Details and deferred
extractions are in [terragame-integration.md](terragame-integration.md).
