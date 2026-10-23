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

## Phase 2 survival review

The current implementation, tests, capture scripts and clean Terrabox working
tree were inspected before extending the existing architecture. Phase 1.1 work
and subsequent repository commits were preserved; no history reset or Terrabox
change was made. The only dependency remains the local Terrabox crate.

`planet::Camp` owns the small crash-clearing overlay, guaranteed nodes, one-time
cache, harvested amounts, shelters, station power and mission progress. It never
mutates World chunks. Original biome/noise and existing landmark placement are
unchanged; formerly empty site regions gain deterministic carbon/crystal.
Inventory and recipes have separate bounded/atomic transaction logic. Equipment
works while carried, avoiding a speculative equipment/ECS layer.

Survival remains a fixed-step system with context supplied by Game. It owns
vitals/time/environmental effects and does not draw or access files. Game owns
state transitions, interactions, objective progression and the small message log.
Exploration accepts a site-query callback so it can reveal the live overlay and
omit depleted resources, while retaining its original base-world API for tests.
The renderer only reads world/expedition state (apart from cache population).

The entry point owns save/load I/O. Encoding is versioned and deterministic;
decoding constructs and validates a replacement game before touching the live
instance. Seed-derived landing/camp geometry is reconstructed rather than
serialized as arbitrary terrain. Persistent mutation records remain separate
from base terrain and survive cache eviction. Writes use create-new temporary
files, flush and same-directory rename. A save failure leaves the game open;
Ctrl+Q allows explicit exit without writing. Loaded games start paused, and
menus/help/crafting/inventory/death have no simulation deadline.

Review checks included transaction rollback, no resource loss on full inventory,
no duplicated emergency cache, exact depletion bounds, tool gating, checked
movement, negative/extreme coordinates, no stale transient scan after load,
complete-night rather than partial-night completion, no offline time, and fuel
freezing alongside simulation. Tests cover save parsing limits and occupied-temp
failure without clobbering the prior save. Basic local saves are not presented
as a cross-filesystem/cloud or universal power-loss protocol.

The scope remains the first loop. Single-cell shelters and linear context checks
are sufficient for a handful of first-session structures; no thread, async
runtime, ECS, base editor or spaceship system was introduced. In-memory history
can grow, saves have documented caps, and narrow HUD/help lines still clip.

## Phase 3 living-world review

Reviewed the current source, tests, README, validation reports, existing generator
and save schema, and Terrabox terminal/timing/buffer/surface APIs before changing
code. Terrabox still provides the complete low-level capability set: RAII terminal
ownership, blocking timed input, resize, FrameClock and clipped buffered cells.
No engine modification, dependency, background worker or replacement framework
was needed. Existing repository history and controls were preserved.

New immutable generation is in `world/profile.rs` and `world/flora.rs`.
Generator 3 applies profile climate shifts to the same integer noise fields;
generator 2 retains exact original classification. The profile influences
oxygen/climate/gravity/exposure, water/vegetation, mineral yield and fauna.
Species/AI (`wildlife.rs`) and regional weather (`weather.rs`) have independent
salted coordinate/ID/tick streams. Weather has no growing cache. Wildlife uses
64 active slots within radius 48; simple local moves resolve in ID order against
terrain, refuge/player cells and occupied positions. Persistent injuries/deaths
and loot are separate from ambient respawning. Active positions are saved too.

Camp retains overlays, adding flora depletion and ruin loot. Exploration receives
the live site query and weather-limited reveal radius; immutable generation never
marks tiles explored. The scanner adds wildlife and environmental contacts, and
M opens a paused survey. Rendering stays in the existing Surface/Canvas path;
AI and exposure rules have no rendering dependency. New items/recipes append to
existing IDs/order rather than shifting old save meanings.

A single Exposure calculation combines planetary base, modest biome offsets,
night, weather, equipment and shelter. Legacy temperature/damage semantics remain
available. The crash shield neutralises hazardous climate/weather locally but
keeps the planet's atmospheric oxygen rules. Energy now gates actions and resting
or consumables restore it without a zero-energy soft lock. Instant lethal plant
injury enters Dead before another action can pause time or use supplies.

Persistence is schema 2 with explicit generator 2/3; real schema-1 fixtures import
on generator 2. Full replacement validation happens before live state changes.
The first overwrite of v1 creates an exact, non-overwriting .v1.bak (LF and CRLF
coverage); conflicting backups/oversized files cause errors. Active and remembered
creatures, plants, ruins, effects and scanned IDs have explicit caps. Weather is
reconstructed from stored time/location, never wall-clock/offline time.

Correctness review covered transaction failures, partial/full depletion, building
on harvested plants, old shelters overlapping the new flora layer, canonical AI
ordering, corpse loot, blocked creature movement, refuge protection, overflow at
world edges, save limits/duplicates and unsupported version rejection. Tests
caught the harvested-plant/shelter validation edge case and guard it permanently.
A full real-time PTY expedition verified the connected exploration/storm/night
loop and persisted dawn completion before loading/continuing.

Representative CPU measurements are recorded in performance-phase3.md. They do
not justify worker threads or additional caches. Remaining design limits are
simple greedy AI, single-cell shelter, permanent flora/mineral depletion and
bounded persistent wildlife records; no galaxy/travel/multiplayer framework was
added. Native runtime validation on the other target platforms is still needed.

## TerraGame foundation extraction

Shared code now lives in the sibling TerraGame workspace. Core owns a bounded
fixed-step clock and exact seed hashing, world owns signed chunk indexing and
the original distance-evicted cache, and terminal owns camera positioning and a
thin Terrabox Session adapter. TerraTopia's public game-facing wrappers, 250 ms
step/four-step bound, terrain generators, survival tick and save schema are
preserved. No planetary gameplay moved into the engine. TerraTopia retains its
save-aware main loop and UI; neither the example application driver nor the
layer composer is forced into it. Persisted wildlife IDs are deliberately left
alone. All existing 53 tests and the actual timed full-night PTY acceptance pass
after integration. See terragame-integration.md for file-level scope.
