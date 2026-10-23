# Phase 4 ecosystem architecture

TerraTopia owns species, needs, interactions, population rules and save records.
TerraGame owns generic generational IDs, arena lookup, spatial indexing, fixed
steps, multirate scheduling, active region keys, bounded FIFO events and local A*.
Terrabox still owns terminal input/output, surfaces, colour, resize and restoration.
There are no new dependencies, threads, runtimes or graphics backends.

## Review and baseline

Inspected all three repositories before implementation, including wildlife,
Camp depletion, terrain/cache, weather, survival, rendering, persistence, tests,
PTY/capture scripts and existing benchmark reports. All working trees were clean.
The existing 53 TerraTopia tests and 14 TerraGame tests passed before changes.
Existing clock, hashing, cache, camera and Session integration was retained.
Species and ecological rules stay in the game; only bounded local navigation was
added to the generic engine. No Terrabox changes were necessary.

Pre-change release sample on this host: TerraTopia cold generation 500 chunks
43.683 ms; 1M warm lookups 19.785 ms; 300 scrolling frames 143.717 ms;
500 scans 71.875 ms; 1,000 one-second wildlife updates 6.192 ms (13 animals);
1,000 movement actions 39.642 ms. TerraGame: 100k arena spawn/remove pairs
2.543 ms; 10k radius queries over 10k objects 191.580 ms; 100k cross-chunk
moves 107.375 ms; 1M scheduler ticks 21.927 ms; 1,000 logical frames 354.153 ms.
These are baseline samples, not direct comparisons to larger Phase 4 workloads.

## Identity and entity lifecycle

The existing persistent ID `(region_x, region_y, slot: u8)` is unchanged.
Slots 0–2 retain their original generation and category rules. Starter slots
3–5 retain their exceptional camp-world-coordinate meaning. New ambient slots
6–17 and birth slots 32–255 are explicit additions; 18–31 are reserved.
Each region's monotonic `next_slot` prevents reuse after death or loot.

Every activated resident, including an untouched animal, has a persistent state
record independent of terrain cache entries. It contains position, health,
aggression, loot status, hunger, energy, fear, age and activity. Discovery IDs and
kill/loot records are never evicted to make room for new populations.

The existing game-facing dense creature vector remains ID-sorted. TerraGame's
`EntityStore<usize>` maps generational runtime handles to that vector's indices;
this preserves the renderer/scanner/combat integration without duplicating full
creatures in an arena. `SpatialIndex` contains living handles only. Movement
updates it immediately; death removes occupancy; dormancy removes arena entries.
Nearby results are sorted by persistent identity, independently of slot reuse.
Corpses retain runtime entries until deactivated/removed, but never block movement.
A stale runtime handle cannot identify a later occupant. Handles are never saved.

## Needs and behaviour

Four categories use ASCII glyphs and colour: grazer `g`, defender `d`, predator
`p`, scavenger `s`; unconsumed corpses use `x`. All artwork is terminal cells.
Hunger/energy/fear are bounded 0–100, age is a saturating counter, and generated
species parameters include lifespan, maturity, birth period, territory radius,
habitat, temperature tolerance and weather resistance. They are lightweight
Rust fields/default generation rules, not a data-file/modding framework.

Explicit states are Idle, Wander, Forage, Eat, Rest, Flee, Defend, Hunt and Return
to territory. Decisions use seed/ID/tick hashes and stable ties, not wall-clock
randomness. Hunger grows slowly; eating lowers it. Movement spends energy;
rest recovers it with hysteresis. Weather, intolerable climates, old age and
starvation can kill animals. Habitat influences placement and movement.

Grazers flee players/predators, seek non-toxic plants and consume the same
`Camp::plants` depletion overlay used by player harvesting. A depleted plant
cannot yield player inventory or another meal, including after chunk eviction.
Grassland/forest grazing also spends a region's finite renewable forage budget.
That budget is distinct from harvestable plants and never creates inventory.
Predators select nearby grazers/scavengers, attack adjacent prey, eat remains,
and stop hunting when satisfied. Pursuit is territory-limited. Scavengers seek
nearby uneaten corpses. Corpse consumption and player loot share one flag, so
remains produce one reward. Defenders retain player proximity/territory combat.
The crash clearing and shelters still protect the player.

## Scheduling, populations and dormant regions

The existing 250 ms survival tick remains authoritative. TerraGame's Scheduler
runs detailed wildlife every four ticks and demographics every 240 ticks.
AI processes living animals in persistent-ID order; no simulation depends on
terminal geometry, rendering frequency or cached chunk order. Off-screen animals
inside the active area can simulate but only visible animals render.

ActiveRegions retains a radius-two square of 32-cell region keys. Individual
records are activated within Manhattan distance 48; movement can extend up to
12 cells beyond that activation boundary before the next refresh. Saves validate
this bounded margin. Normal populations depend on habitat and abundance; the
1,024 active-record ceiling is a stress capacity, not a guaranteed density.

Each remembered region has four desired living counts, a forage budget (0–120),
a habitat-derived capacity (6–24), last coarse epoch and birth sequence. Fed,
mature parents of the same category can produce at most one child per region
per minute, subject to food, population, memory, occupancy and identity limits.
Individuals age and die; corpses remain meaningful food/loot. Parameters are
intentionally simple; no genetics or colony management exists.

Dormant records retain their exact last individual state and do not move, hunt,
feed or execute AI. One coarse operation per dormant region per minute adjusts
food, bounded growth and rare mortality (absolute 100-minute epochs, with a small
population reserve). This is an approximate demographic model; individual needs
and age freeze while dormant. No planetary list of every possible creature exists.

On activation, desired counts reconcile against retained living records in ID
order. Coarse deaths choose highest IDs, leaving persistent corpses. New animals
receive unused birth IDs and deterministic suitable homes. Known kills stay dead;
looted remains and harvested plants stay consumed. Conflicting homes are skipped.
Reconstruction can undershoot a target when habitat, identity or record limits
prevent a valid birth. It never fabricates resources to meet a target.

Bounds are 1,024 active records, 10,000 persistent creature records and 10,000
region summaries (the region bound also accommodates old saves containing kills
in many distinct regions). New region allocation reserves record headroom.
At a ceiling the game stops adding residents; it does not forget deaths or
recycle identities. Existing exploration/plant/save bounds still apply. Region
activation scans bounded remembered records; it is not constant-time streaming.
Identical simulation ticks/activation histories replay identically. Choosing to
keep a region detailed instead of dormant intentionally uses different rules;
this is not a claim of physical equivalence between detail levels.

## Navigation and events

Movement first tries cardinal steps ranked by goal, escape distance or habitat.
A blocked/non-progressing goal triggers TerraGame's deterministic local A* with
radius 12, at most 64 expansions per request and 512 across all animals per tick.
Occupied targets are approached via a free adjacent goal. Searches retain only
small bounded local maps/heaps; large routes are not allocated every tick.
Only the next step is used. Each request observes current terrain, shelters and
occupancy, so there is no stale cached path to invalidate after a building or
resource change. A budget-starved creature can use a safe local step or wait;
there is no planetary navigation mesh or global path guarantee.

A 4,096-entry TerraGame EventQueue carries committed gameplay facts: spawn,
death, attack, eating, resource consumption, population and weather changes.
Interaction transactions commit before emission; queue backpressure cannot undo
or duplicate a kill/meal. FIFO order is stable (attack, death, eating). Overflow
increments an observable counter. Game consumes events at the tick boundary;
the separate bounded presentation-message log carries warnings/combat text.
There is no unrestricted event bus or saved pending event replay.

## Save compatibility

Schema 3 appends five needs/activity fields to creature records and adds
ECOSYSTEM time and POPULATIONS rows. Generators 2 and 3 are unchanged. Schemas
1 and 2 remain accepted; old tuple IDs keep their original meanings, needs get
explicit defaults, saved actors become retained residents and summaries are
constructed from those records. Genuine pre-change schema-1 and schema-2
fixtures cover migration. First overwrite preserves an exact `.v1.bak` or
`.v2.bak`; an existing conflicting backup fails without replacing the save.

The complete replacement game is validated before becoming live: versions,
generator, byte/entry limits, booleans, coordinate arithmetic, creature
prototypes/health/needs/leashes, active duplicate IDs/positions, reference
consistency, birth sequences, population limits and simulation time. Runtime
handles/index/scheduler state are reconstructed. Weather derives from stored
seed/time/location. Saves are at most 8 MiB and use the existing exclusive
same-directory temporary write, sync and rename. There is no offline time or
universal power-loss/filesystem guarantee. Loaded games start paused.

## Presentation and current limits

The HUD retains inventory, crafting, scanner, weather, survival and controls.
Visible predators add an explicit warning. M's paused inspection lists creature
activity, health and needs. R still records discoveries. Resizing only changes
surfaces/cache access, never ecosystem state. Only visible creatures draw.

Current limits: finite 224 birth-ID candidates per region, hard persistent
ceilings, permanent harvestable-plant depletion, simplified corpse lifetime and
coarse mortality, retained frozen individual records, no migration beyond local
territories, no fairness rotation for exhausted path budgets, and generated
species/defaults rather than detailed ecological calibration. Long tests cover
both controlled demographic bounds and one unassisted 30-minute starter habitat;
they are not a claim of balance on every seed for unlimited play.

See [performance-phase4.md](performance-phase4.md) and
[validation-phase4.md](validation-phase4.md) for measured evidence and next steps.
