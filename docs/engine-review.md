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

## Phase 1.1 review

Re-inspected the current modules, baseline tests, native engine dependency and
Terrabox capture/smoke tooling before extending the game. Existing state, input,
world, camera and rendering ownership remain. No Terrabox changes, dependencies,
threads or new rendering backend were needed.

Landmark placement is pure coordinate logic in `world/landmark.rs`; tiles carry
only the optional feature kind. Expedition bitsets and discovered-site anchors
live in `exploration.rs`, independently of the terrain cache. Renderer reads
visibility without modifying it and skips terrain lookups for unknown cells.
The game reveals on landing, successful movement and explicit scanner actions.
Scan reports deduplicate site anchors, order signals by distance with stable
coordinate ties, and retain a labelled last-scan summary. Fixed-step survival
and menu/pause timing are unchanged. Geological features remain decorative,
preserving the original collision policy.

Generation measurements justified sharing each chunk's octave lattice values
and interpolation weights. The original coordinate sampler remains an independent
reference and tests compare every tile in selected positive, negative and
extreme chunks. This preserves exact fixed-point rounding and existing biomes.
Warm cache access now uses one lookup, and hit/miss/eviction counters expose
behaviour; counter maintenance has a small cost. The 128-chunk policy is retained.
See `performance.md` for measured tradeoffs rather than a general speed claim.

Overflow review: site anchors use i128 before conversion; partial edge regions
with unrepresentable anchors produce no site. Reveal loops use checked coordinate
addition. Visibility bounds differences before squaring, so opposite world ends
cannot overflow distance calculations. Nearby report ordering only squares
small local distances. Screenshot inspection prompted one palette contrast fix;
the report panel moves to the left at the right world edge to avoid hiding `@`.

Remaining boundaries: exploration masks/site records grow with travelled area;
there is no persistence. Sensor radii ignore terrain occlusion. A scan's full
visibility lasts until a successful move, with no real-time decay or cooldown.
The panel displays only the three nearest signals and can obscure map cells,
but not the player. HUD/status text still clips on narrow terminals.
