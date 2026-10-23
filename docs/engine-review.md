# Terrabox integration review

Inspection date: 23 October 2026. TerraTopia had no source files or commits at
entry. Terrabox's working tree was clean. Both repositories and their Git
metadata are preserved; all implementation changes are inside TerraTopia.

Reviewed the workspace manifests and module/API inventory, core engine source
(lifecycle, buffers, cells, styles, input, timing and surfaces), Unix/Windows
backend entry points, TerraTUI's public layer, architecture/engine/validation and
compatibility documentation, examples (including animation, resize, lifecycle,
keyboard, Unicode and colours), tests and PTY smoke tooling. TerraTUI supplies
widgets/application structure but is not needed for this game.

| Existing capability | Integration |
| --- | --- |
| `Terminal::new` and backend RAII | Own raw mode, alternate screen, terminal connection and restoration |
| `wait_event(Option<Duration>)` | Blocking menu/pause input, timed playing waits |
| `Event`, `KeyEvent`, `Key`, modifiers | Normalize keyboard input without another backend |
| `FrameClock` | Deadline scheduling, pause/resume, elapsed samples |
| `Size`, resize events | Recalculate viewport, retain world and player |
| `Surface`, `Canvas`, `Rect` | Reusable offscreen frame with clipped map and overlays |
| `Cell`, text width / grapheme handling | Native Unicode support; initial game uses ASCII single-cell tiles |
| `Style`, indexed colours | Standard palette foregrounds, backgrounds and emphasis |
| Buffered `present` | Emits changed rows and restores requested cursor state |
| Existing PTY tests and smoke scripts | Pattern for keyboard, resize, idle and terminal restoration checks |

Terrabox's FrameClock schedules from actual poll time and skips missed deadlines;
it is not a fixed-step simulation accumulator. The game owns a separate bounded
accumulator. Surface scrolling is available but initially unnecessary: a logical
viewport is redrawn on movement and the engine already compares front/back rows.
Capability probes, mouse, optional Kitty graphics, notifications, external command
handoff and TerraTUI windows are intentionally unused.

Missing game-specific systems were coordinate noise, climate classification,
chunks/cache eviction, collision, a following camera, game states, a fixed-step
simulation accumulator and HUD rules. These belong in TerraTopia and are now
implemented there. No missing low-level engine capability was found, so no
Terrabox change or new dependency is proposed. There is no unsafe Rust in the
game; the existing engine owns platform-specific operations.

Post-implementation review: seed is immutable within a World to prevent stale
cached terrain; only hashing wraps integers, movement checks addition, camera
clamps with i128 intermediates, and chunk coordinates use Euclidean division.
Chunk construction is private to the cache so arbitrary invalid chunk keys are
not a public API. Noise scales are internal fixed values. The cache enforces its
bound before allocating another chunk; its farthest-entry scan is only 128 entries
and happens on misses. Pure generation has no cache or terminal dependencies.
The deterministic landing search is bounded and does not warm thousands of
chunks. No speculative future systems were added.

Known performance boundary: traversing a viewport spanning more than 128 chunks
can thrash the cache. Chunk creation and frame construction remain synchronous.
Normal terminal dimensions fit comfortably within the bound. A future measured
need could justify adaptive capacity or a generation worker; neither needs an
engine API change today.
