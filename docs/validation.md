# Phase 1.1 validation

Date: 23 October 2026. Host: Linux x86_64, stable rustc 1.97.1 / cargo 1.97.1.
Existing architecture and Git history are preserved. Terrabox is unchanged and
its working tree remains clean. No new dependencies were introduced.

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | Pass: 22 tests (12 existing, 10 exploration tests) |
| `cargo build --release` | Pass |
| `cargo check --target x86_64-pc-windows-gnu` | Pass; compilation cross-check only |
| `python3 scripts/smoke-terminal.py` | Pass in Unix PTYs for Q, Escape and Ctrl+C exit runs |
| `cargo run --release --example benchmark` | Executed; five-sample comparison recorded |
| `python3 scripts/capture-terminal.py` | Pass: two actual xterm PNG captures |
| `git diff --check` | Pass |

The original tests retain deterministic seed/order/cache checks, positive and
negative chunk boundaries, i64 extremes, biome classification, coherence,
collision/cardinal movement, cache eviction, camera/resize/tiny dimensions,
input/modifiers, states, bounded simulation and landing-site checks.

New coverage checks:

- Every tile in selected positive, negative and extreme chunks matches the
  independent coordinate generator after lattice reuse (three seeds).
- All six landmark kinds occur, including three resource types with distinct
  glyphs and colours; features cross positive and negative chunk boundaries.
- Generating terrain does not explore it; exploration survives cache eviction.
- Circular radius-7 sight and radius-20 scanning, correct edge exclusions,
  scan deduplication and visibility at opposite i64 world limits.
- Remembered versus unknown versus visible rendering, read-only fog rendering,
  scan input, paused scan suppression, expedition reset and cache counters.
- The scan panel does not hide the player at the right world boundary.

The actual release-binary PTY script enters menu/landing, confirms fog glyphs,
scans, checks the resource legend/report and reduction in unknown map output,
checks repeat scans report zero new cells/sites, moves with WASD and arrows,
pauses, confirms movement/scanning are ignored, resizes to 20x6 then 100x30,
resumes, scans again, starts a fresh expedition and confirms its knowledge reset.
Each run asserts successful exit, exact termios restoration, alternate-screen
exit and cursor restoration. Q, Escape and Ctrl+C are tested separately.

All nine 600ms menu, playing-idle and paused samples recorded **0 CPU ticks**
through Linux `/proc` and **no terminal output**. Playing still wakes four times
per second; menu/pause block without deadlines. These are short smoke checks.

Two real Linux xterm screenshots were captured using the final release binary
and inspected visually. They show the scan and the remembered terrain after
movement, with named silica deposits and geological signals. Capture inspection
exposed poor wasteland contrast; its background is now black. Landmark glyphs
also use black backgrounds for contrast across the underlying biomes. The PNGs
contain no personal information. Reproduction and emulator details are in
[the capture guide](../assets/screenshots/README.md).

No physical keyboard session, visible flicker/latency benchmark, native Windows,
macOS, SSH or Android/Termux runtime tests were performed. Xterm capture/input
uses synthetic X key events; PTY tests use real terminal byte input. Neither
certifies all terminals or native console hosts. Windows was cross-checked only.
Forced termination/abort recovery is not claimed.

See [performance.md](performance.md) for measurements, including the absence of
a demonstrated scrolling speedup, and [engine-review.md](engine-review.md) for
ownership and overflow review.

Limits: static vitals, no mining/inventory/persistence, non-occluded sensor sight,
unbounded expedition knowledge, a three-signal last-scan panel, narrow-screen
clipping and huge-viewport cache thrashing. Recommended next milestone: resource
collection/inventory plus oxygen and basic environmental hazards, alongside
native runtime checks on the remaining target platforms.
