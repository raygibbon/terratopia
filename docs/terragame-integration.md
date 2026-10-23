# TerraGame integration

The sibling `../terragame/` workspace supplies three local dependencies. Keep it
beside TerraTopia and Terrabox when building. No third-party dependency or
Terrabox source change was needed. Existing uncommitted Phase 3 work and Git
history were preserved.

| File | Change |
|---|---|
| Cargo.toml / Cargo.lock | Local core/world/terminal dependencies |
| src/game.rs | Generic FixedStep replaces the private accumulator |
| src/world/seed.rs | Exact original mix/hash re-exported from core |
| src/world/chunk.rs | Signed ChunkLayout indexing |
| src/world/mod.rs | Generic bounded distance ChunkCache |
| src/camera.rs | Engine camera positioning behind existing wrapper |
| src/main.rs | Terrabox lifecycle/Surface through engine Session |

The 32-cell layout, hash constants, generator 2/3, schema-2 saves, schema-1 import,
old backup protection, seed values and simulation rules remain unchanged. The
saved survival tick stays authoritative; partial wall time and the helper's
wall-step counter are transient. All 53 existing tests and the timed full-night
PTY acceptance passed after integration.

Biomes, atmosphere, oxygen, recipes, inventory, species, objectives and terrain
remain game-owned. The working renderer still draws Terrabox Canvas directly.
Moving saved region-slot wildlife identities into a new entity arena or changing
its update order would need a separate measured migration and compatibility
plan; neither was forced into this extraction.

See the TerraGame README, architecture, validation and performance reports in
`../terragame/` for the reusable APIs, two runnable terminal examples, benchmarks
and remaining limitations.

## Phase 4

Wildlife now uses EntityStore for transient identity/index lookup, SpatialIndex
for occupancy and nearby contacts, Scheduler for one-second AI/minute demographics,
ActiveRegions for activation keys and EventQueue for bounded gameplay facts.
Terragame-world's new navigation module provides budgeted local A*; species,
resource/food interactions, population summaries, save migration and terminal
inspection remain TerraTopia's responsibility. See [ecosystem.md](ecosystem.md).
