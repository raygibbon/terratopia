# Phase 1 validation

Date: 23 October 2026. Host: Linux x86_64, stable rustc 1.97.1 / cargo 1.97.1.
All source changes are inside TerraTopia. Terrabox remains unchanged.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | Pass: 12 system tests |
| `cargo build --release` | Pass |
| `cargo check --target x86_64-pc-windows-gnu` | Pass; cross-check only |
| `python3 scripts/smoke-terminal.py` | Pass for Q, Escape and Ctrl+C exit runs |
| CLI `--help`, malformed negative seed | Usage/help and error behaviour checked |
| `git diff --check` | Pass |
| Terrabox working-tree status | Clean; no source changes |

The Rust tests exercise seed/coordinate reproducibility independent of cache
capacity and traversal order, direct versus chunk generation across positive and
negative boundaries, extreme i64 coordinates, all seven generated/classified
biomes, coherent neighbour samples, collision/cardinal movement, cache bounds and
distance eviction, camera centring and edge clamping, resize/tiny dimensions,
input/modifiers, menu/new-world/pause/restart/quit, paused simulation and bounded
catch-up, surface/HUD/player/overlay rendering, and deterministic walkable landing
sites without cache prewarming.

The PTY script launches the actual release binary with seed 827193 and a
controlling terminal, following Terrabox's documented PTY smoke approach. Each
run enters the menu and landing screen, lands, moves with WASD and arrows, pauses,
checks that paused movement is ignored, resizes to 20x6 and back to 100x30,
resumes, moves again and quits. Exact termios restoration, alternate-screen exit,
cursor restoration and successful exit are asserted. A small-window message
initially clipped its required dimensions at 20 columns; the test exposed this
and the shortened message now passes.

Across three runs, the script sampled main-menu, playing-idle and paused states
for 600ms each. All nine samples recorded **0 CPU ticks** through Linux `/proc`
and **no terminal output**. These short checks support the idle policy; they
are not a long-duration performance benchmark. Playing still wakes four times
per second to update its simulation clock; menu/pause block without deadlines.

No physical interactive keyboard/font/colour/flicker sign-off was performed in a
visible emulator. PTY input and resize tests are automated behavioural tests,
not visual certification. Native Windows, macOS, SSH and Android/Termux runtime
tests were not performed. The Windows result checks Rust compilation only.
Forced termination/abort recovery was not claimed.

Remaining limits: vitals are static, days have no lighting effects, there is no
persistence or resource interaction, disconnected land can constrain walking,
long HUD text clips in narrow terminals, and huge viewports can exceed the
128-chunk cache. The implementation review is recorded in
[engine-review.md](engine-review.md). Recommended next milestone: oxygen and
basic environmental hazards, followed by inventory and resource collection,
alongside native terminal validation on the remaining platforms.
