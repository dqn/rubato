# Porting TODO — Mechanical Line-by-Line Translation

Dependency graph order. Each module is ported only after its dependencies are complete.

## Phase 1: Core Foundation (~11,178 lines)

Zero internal dependencies. Port these first.

- [x] `bms.model` (19 files, 8,070 lines) — BMS format parser
- [x] `bms.table` (10 files, 3,108 lines) — LR2 course table parser

## Phase 2: Format Variants (~1,802 lines)

Depends on: bms.model

- [x] `bms.model.bmson` (16 files, 526 lines) — BMSON parser
- [x] `bms.model.osu` (9 files, 1,276 lines) — osu! format converter

## Phase 3: Low-level Subsystems (~12,050 lines)

Isolated subsystems with minimal internal deps.

- [x] `beatoraja.exceptions` (1 file, 7 lines) — Exception definitions
- [x] `beatoraja.system` (1 file, 139 lines) — File utilities
- [x] `tool.util` (1 file, 121 lines) — Generic utilities
- [x] `beatoraja.controller` (3 files, 762 lines) — Gamepad management
- [x] `beatoraja.external.DiscordRPC` (4 files, 634 lines) — Discord RPC
- [x] `beatoraja.input` (8 files, 4,188 lines) — Keyboard/MIDI input
- [x] `beatoraja.audio` (14 files, 7,086 lines) — Audio playback (Kira)
- [x] `tool.mdprocessor` (9 files, 2,264 lines) — Download/song processing

## Phase 4: Configuration & Central State (~26,290 lines)

Hub module — most others depend on this.

- [x] `beatoraja.config` (4 files, 2,582 lines) — Config definitions
- [x] `beatoraja` root (44 files, 23,708 lines) — Central state/data classes

## Phase 5: Pattern & Gameplay (~17,692 lines)

Core gameplay logic. Depends on: config, model, input, audio.

- [x] `beatoraja.pattern` (14 files, 4,108 lines) — Lane/note shuffle
- [x] `beatoraja.play` (23 files, 13,584 lines) — Judge, gauge, game loop
  - [x] `beatoraja.play.bga` (5 files, 1,802 lines) — BGA playback

## Phase 6: Skin System (~15,594 lines)

Multi-format skin rendering. Depends on: config, model, play.

- [x] `beatoraja.skin` base (34 files, 15,594 lines) — Skin rendering engine
  - [x] `beatoraja.skin.json` (11 files, 5,456 lines) — JSON skin loader
  - [x] `beatoraja.skin.lr2` (10 files, 6,482 lines) — LR2 skin loader
  - [x] `beatoraja.skin.lua` (5 files, 2,480 lines) — Lua skin loader
  - [x] `beatoraja.skin.property` (13 files, 8,216 lines) — Property binding

## Phase 7: Screen Implementations (~11,900 lines)

UI screens. Depends on: skin, config, play.

- [x] `beatoraja.select` (13 files, 8,386 lines) — Song select screen
  - [x] `beatoraja.select.bar` (17 files, 3,514 lines) — Bar rendering
- [x] `beatoraja.result` (7 files, 3,122 lines) — Result screen
- [x] `beatoraja.decide` (2 files, 172 lines) — Decide screen

## Phase 8: Advanced Features (~15,946 lines)

Optional/peripheral features.

- [x] `beatoraja.ir` (14 files, 3,572 lines) — Internet ranking
- [x] `beatoraja.external` (7 files, 2,076 lines) — OBS, webhooks
- [x] `beatoraja.obs` (2 files, 1,502 lines) — OBS WebSocket
- [x] `beatoraja.modmenu` (15 files, 8,468 lines) — In-game mod menu
- [x] `beatoraja.stream` (3 files, 402 lines) — Stream commands

## Phase 9: Launcher (~9,210 lines)

Standalone GUI. Can be deferred.

- [x] `beatoraja.launcher` (21 files, 9,210 lines) — Settings GUI (egui)

## Phase 10: Remaining Modules (~2,726 lines)

Untranslated Java files not covered by Phase 1–9.

- [x] `beatoraja.song` (8 files, 2,206 lines) — Song data model & DB accessor
- [x] `beatoraja.controller` (3 files, 381 lines) — Lwjgl3 gamepad (LibGDX-dependent)
- [x] `beatoraja.system` (1 file, 139 lines) — RobustFile I/O utility

## Phase 11: Integration & Wiring

Replace stubs with real cross-crate imports. No new translation — just connecting existing code.

- [x] Replace `SongData` stubs with `beatoraja-song` import (ir, skin, play, result)
- [x] Replace `MainController` internal stubs in `beatoraja-core` with real import
- [x] Replace `TextureRegion`/`Texture`/`Color`/`Pixmap`/`Rectangle` stubs in `beatoraja-select`/`beatoraja-result` with `beatoraja-skin` import
- [x] Replace `SkinProperty` constants in `beatoraja-external` with `beatoraja-skin` import
- [x] Replace `MessageRenderer` stub in `beatoraja-stream` with `beatoraja-core` import
- [x] Replace `SoundType`/`bms_model::Mode` stubs in `beatoraja-select` with real imports
- [x] Remove 11 unused stubs from `beatoraja-core`, 13 from `beatoraja-play`
- [x] Add 30+ getter methods to `SongData` and `ScoreData` for stub API compatibility
- [x] Add `beatoraja-song` dependency to 7 downstream crates
- [x] Resolve circular dependency issues (documented: core↔song, core↔skin, core↔play, play↔skin, input↔core, audio↔core)
- [x] Verify: all 66 tests pass, zero clippy warnings, clean `cargo fmt`

## Phase 12: Binary Entry Point

Create an executable binary target.

- [x] Add `[[bin]]` target to workspace (`beatoraja-bin` crate)
- [x] Implement `main()` wiring CLI args → Config → MainController → game loop
- [x] Replace LibGDX `Lwjgl3Application` with winit event loop (Bevy rendering in Phase 13)

## Phase 13: External Library Integration

Replace `todo!()` stubs with real library calls (~377 `todo!()` total).

- [ ] LibGDX rendering → Bevy (TextureRegion, SpriteBatch, ShaderProgram, etc.)
- [ ] JavaFX UI → egui (launcher views, ~40 `todo!()`)
- [ ] LuaJ → mlua (Lua skin loader, ~40 `todo!()`)
- [ ] PortAudio → cpal/Kira (audio driver, ~20 `todo!()`)
- [ ] FFmpeg → ffmpeg-next (BGA video processing)
- [ ] javax.sound.midi → midir (MIDI device enumeration)
- [ ] 7z extraction → sevenz-rust

## Phase 14: Remaining Stub Unification

Resolve type stubs that Phase 11 could not replace due to circular dependencies or API mismatches.

### Circular Dependency Resolution

Extract shared types into a low-level crate to break cycles.

- [x] Create `beatoraja-types` crate with shared types (Config, PlayerConfig, PlayModeConfig, Resolution, AudioConfig, IRConfig, SkinConfig, PlayConfig, ScoreData, CourseData, ReplayData, ClearType, BMKeys, Validatable)
- [x] Replace `beatoraja-core` 14 modules with `pub use beatoraja_types::*` re-exports
- [x] Replace `beatoraja-input` Config/Resolution/PlayModeConfig/KeyboardConfig/ControllerConfig/MidiConfig/MidiInput/MidiInputType/MouseScratchConfig/PlayerConfig stubs with `beatoraja-types` import
- [x] Replace `beatoraja-audio` Config/AudioConfig stubs with `beatoraja-types` import
- [x] Add compatibility getter methods to `beatoraja-types` for stub API compatibility
- [x] Update beatoraja-input callers: Resolution field access → method calls, MidiInputType variant names
- [x] Verify: all 66 tests pass, zero clippy warnings, clean `cargo fmt`
- [ ] Replace `beatoraja-play` stubs for TextureRegion/Texture with `beatoraja-types` import (rendering stubs — deferred to Phase 13)
- [x] Replace remaining stubs in downstream crates (Config, PlayerConfig, ScoreData, etc.)

### API Incompatibility Resolution

Align stub APIs with real type APIs across all crates.

- [x] Unify Config/PlayerConfig field types (`String` vs `Option<String>`, `f32` vs `i32`)
- [x] Unify Resolution type (struct with `f32` fields vs enum with `i32` methods)
- [x] Unify SongDatabaseAccessor (struct in stubs vs trait in real implementation — completed in Phase 15c)
- [x] Unify BMSPlayerInputProcessor parameter types (`i32` vs `usize` — completed in Phase 15c)
- [x] Unify ScoreData method signatures (`set_player(String)` vs `set_player(Option<&str>)`)
- [x] Update all callers to match unified APIs
- [x] Reduce `stubs.rs` files to rendering-only + circular dep stubs

### Stubs Replaced in Downstream Crates

- [x] `md-processor`: Config stub → `pub use beatoraja_core::config::Config`
- [x] `beatoraja-ir`: `convert_hex_string` stub → `pub use bms_model::bms_decoder::convert_hex_string`
- [x] `beatoraja-result`: IRConfig, IRResponse, IRScoreData, IRCourseData, IRChartData, RankingData, RankingDataCache → real imports from `beatoraja-core`/`beatoraja-ir`
- [x] `beatoraja-external`: Config, PlayerConfig, ScoreData, SongData, ReplayData → real imports from `beatoraja-core`/`beatoraja-song`
- [x] `beatoraja-modmenu`: Config, PlayConfig, PlayModeConfig, ScoreData, Version → real imports from `beatoraja-core`
- [x] `beatoraja-select`: Config, SongPreview, PlayerConfig, PlayModeConfig, PlayConfig, KeyboardConfig, ControllerConfig, MidiConfig, ScoreData, AudioConfig, Resolution → real imports from `beatoraja-core`
- [x] `beatoraja-launcher`: BMSPlayerMode, Version → real imports from `beatoraja-core`
- [x] Verify: all 66 tests pass, zero clippy warnings, clean `cargo fmt`

### Remaining Stubs (Cannot Replace)

Stubs that remain due to structural mismatches or external library dependencies:

- ~~**Structural mismatch:** TableData/TableFolder/TableAccessor — CourseData cascade~~ → resolved in Phase 15g
- **Rendering:** TextureRegion/Texture in `beatoraja-play` (skin→play circular dep; deferred to Phase 13)
- **Lifecycle stubs (trait-ified):** MainController/PlayerResource stubs remain but implement `MainControllerAccess`/`PlayerResourceAccess` traits (Phase 15d). MainState stubs remain as-is — `beatoraja-core` already defines `MainState` trait; downstream stubs have crate-specific APIs (timers, skin, input) that don't converge to a shared trait
- **External libraries:** LibGDX rendering types (Phase 13), ImGui/egui types (Phase 13)
- ~~**Platform-specific:** Twitter4j/AWT clipboard~~ → resolved in Phase 15e (Twitter: graceful bail, clipboard: arboard, PortAudio: cpal, monitors: CoreGraphics FFI)

Resolved in Phase 15a-d: ~~SongData circular dep~~ → moved to `beatoraja-types`. ~~SkinType/GrooveGauge~~ → moved to `beatoraja-types`. ~~SongDatabaseAccessor/IRConnection struct-vs-trait~~ → replaced with real traits. Resolved in Phase 15g: ~~TableData/CourseData cascade~~ → unified CourseData/TrophyData/CourseDataConstraint types, replaced stubs with real imports. Resolved in Phase 15e: ~~Twitter4j/AWT clipboard/PortAudio/Monitor enumeration~~ → replaced with Rust equivalents (arboard, cpal, CoreGraphics FFI).

## Phase 15: Structural Refactoring & Remaining Stubs

Depends on: Phase 13 (rendering stubs), Phase 14 (type unification).
Resolve all non-rendering stubs that remain due to structural mismatches, circular dependencies, or missing platform equivalents.

### 15a: Circular Dependency — SongData Extraction

Move `SongData` into `beatoraja-types` to break core→song circular dep.

- [x] Move `SongData` struct from `beatoraja-song` to `beatoraja-types` (keep DB accessor in `beatoraja-song`)
- [x] Move `SongInformation` struct from `beatoraja-song` to `beatoraja-types`
- [x] Move `IpfsInformation` trait from `md-processor` to `beatoraja-types` (breaks cycle: types→md-processor→core→types)
- [x] Replace `SongData` stub in `beatoraja-core/stubs.rs` with `pub use beatoraja_types::SongData`
- [x] Replace `SongData` stubs in `beatoraja-select`, `beatoraja-modmenu`, `beatoraja-launcher` with `beatoraja-types` import
- [x] Update callers for API changes: `Option<String>` → `String` fields, `get_full_title(&mut self)` → `full_title(&self)`, `set_path(Option)` → `set_path_opt`/`clear_path`
- [x] Remove unused `SongDataExt` workaround trait from `beatoraja-ir`
- [x] Verify: all 66 tests pass, zero clippy warnings, clean `cargo fmt`

### 15b: Circular Dependency — SkinType / GrooveGauge Extraction

Move enum definitions into `beatoraja-types` to break skin/play→core cycles.

- [x] Move `SkinType` enum from `beatoraja-skin` to `beatoraja-types` (added Copy, Default, Hash derives; PascalCase variants)
- [x] Move `GrooveGauge` struct + `Gauge` + `GaugeModifier` + `GaugeProperty` + `GaugeElementProperty` from `beatoraja-play` to `beatoraja-types`
- [x] Move `GrooveGauge::create` to free function `create_groove_gauge` in `beatoraja-play` (depends on `BMSPlayerRule`)
- [x] Replace SkinType stubs in `beatoraja-types/stubs.rs`, `beatoraja-select/stubs.rs`, `beatoraja-modmenu/stubs.rs` with `pub use` re-exports
- [x] Replace GrooveGauge stub in `beatoraja-types/stubs.rs` with `pub use` re-export
- [x] Update `skin_config.rs` SkinDefault table: UPPER_SNAKE_CASE → PascalCase, `get_default(usize)` → `get_default(i32)`
- [x] Update `player_config.rs` for `get_max_skin_type_id() -> i32` return type
- [x] Update `beatoraja-modmenu/skin_menu.rs` variant names and `get_id() -> i32` casts
- [x] Add `beatoraja-types` dependency to `beatoraja-skin`, `beatoraja-play`, `beatoraja-modmenu`
- [x] Verify: all 66 tests pass, zero clippy warnings, clean `cargo fmt`

### 15c: Struct-vs-Trait Unification

Define shared traits in `beatoraja-types`, implement in real crates.

- [x] `SongDatabaseAccessor`: define trait in `beatoraja-types` (with `: Send`), implement in `beatoraja-song`, replace struct stubs in `beatoraja-select`/`beatoraja-external`. Also moved `FolderData` to `beatoraja-types`.
- [x] `IRConnection`: replace struct stubs with real trait from `beatoraja-ir` in `beatoraja-select` (`Box<dyn IRConnection>`) and `beatoraja-result` (`Arc<dyn IRConnection>`). Also replaced `LeaderboardEntry`, `LR2IRConnection`, `LR2GhostData`, `IRScoreData`, `IRChartData`, `IRPlayerData`, `IRResponse`, `IRTableData` stubs with real imports.
- [x] `BMSPlayerInputProcessor`: unify analog method parameter types (`i32` → `usize`) in stubs, update callers in `music_select_key_property.rs`
- [~] `TableDataAccessor` / `TableAccessor`: added getters to real `TableData`/`TableFolder` in `beatoraja-core` (preparation). **Cannot replace stubs**: real `TableData.course` uses `beatoraja-types::CourseData` but select uses its own `CourseData` stub with different fields (`song` vs `hash`, `String` vs `Option<String>`, `f64` vs `f32`). Replacing requires cascading `CourseData`/`TrophyData`/`CourseDataConstraint` changes across ~10 files. Deferred to future phase.
- [x] Verify: all 66 tests pass, zero clippy warnings, clean `cargo fmt`

### 15d: MainController / PlayerResource / MainState Lifecycle

Define trait interfaces in `beatoraja-types` for the "god objects" so downstream crates use traits instead of concrete stubs.

- [x] Define `MainControllerAccess` trait in `beatoraja-types` (config access, screen transitions, save, exit, update_song, player_resource access)
- [x] Define `PlayerResourceAccess` trait in `beatoraja-types` (config, score data, song data, replay data, course data, gauge, state queries)
- [x] Move `MainStateType` enum from `beatoraja-core` to `beatoraja-types` (re-export via `pub use` in core)
- [x] Implement traits on real types in `beatoraja-core` (`MainController`, `PlayerResource`)
- [x] Add trait impls to existing stubs in 8 downstream crates: select, ir, obs, result, decide, external, modmenu (stream has no MainController stub)
- [x] Provide `NullMainController` and `NullPlayerResource` default impls in `beatoraja-types`
- [~] `MainStateAccess` trait: deferred — existing `MainState` trait in `beatoraja-core` already serves this purpose; downstream stubs vary too much for a unified trait
- [x] Verify: all 66 tests pass, zero clippy warnings, clean `cargo fmt`

### 15g: TableData / CourseData Cascade Unification

Resolve the TableData/TableAccessor stubs blocked by CourseData field mismatches (deferred from Phase 15c).

- [x] Unify `CourseData` fields across stubs and real type: `song` → `hash`, `String` → `Option<String>`, `f64` → `f32`
- [x] Update `TrophyData` / `CourseDataConstraint` stubs to match real types
- [x] Replace `TableData` / `TableFolder` / `TableAccessor` stubs in `beatoraja-select` with real imports from `beatoraja-core`
- [x] Replace `TableDataAccessor` / `DifficultyTableAccessor` / `CourseDataAccessor` stubs with real imports from `beatoraja-core`
- [x] Add missing getter/setter methods to real `CourseData` and `TrophyData` in `beatoraja-types`
- [x] Add `Send + Sync` bounds to real `TableAccessor` trait in `beatoraja-core`
- [x] Update `BMSSearchAccessor` to implement real `TableAccessor` trait (`read` → `Option<TableData>`, `write` → `&mut TableData`)
- [x] Update `table_bar.rs`: `get_url()` → `get_url_opt()` for `Option<&str>` return
- [x] Update `grade_bar.rs`: `TrophyData` rates f64 → f32 in `qualified()` arithmetic
- [x] Verify: all 66 tests pass, zero clippy warnings, clean `cargo fmt`

### 15e: Platform-Specific Replacements

Replace or remove stubs with no direct Java equivalent.

- [x] Twitter4j (`beatoraja-external`, `beatoraja-launcher`): replaced `todo!()` with `anyhow::bail!()` — graceful error instead of panic (Twitter API has no Rust equivalent; kept struct API for compatibility)
- [x] AWT clipboard (`beatoraja-external`): replaced with `arboard` crate + `image` crate for cross-platform clipboard image copy
- [x] Text clipboard (`beatoraja-launcher`): replaced with `arboard` crate for cross-platform text clipboard
- [x] PortAudio device enumeration (`beatoraja-launcher`): replaced with `cpal` host/device listing
- [x] Monitor enumeration (`beatoraja-launcher`): replaced with CoreGraphics FFI on macOS (winit 0.30 `available_monitors()` requires `ActiveEventLoop`; proper winit-based enumeration in Phase 13 egui integration)
- [x] Verify: all 66 tests pass, zero clippy warnings, clean `cargo fmt`

### 15f: Final Stub Cleanup

Remove all remaining `stubs.rs` files or reduce to zero non-rendering stubs.

- [ ] Audit each crate's `stubs.rs` — remove stubs that are now unused
- [ ] Move Phase 13 rendering stubs (if any remain) into dedicated `rendering_stubs.rs` to separate from structural stubs
- [ ] Verify: no non-rendering stubs remain outside `rendering_stubs.rs`
- [ ] Verify: all tests pass, zero clippy warnings, clean `cargo fmt`

---

## Testing Checkpoints

| After Phase | What you can test |
|-------------|-------------------|
| 1 | BMS parsing independently (Golden Master) |
| 2 | All format variants (BMS, BMSON, osu!) |
| 3 | Input/audio subsystems |
| 5 | Full gameplay logic with judge calculations |
| 6 | Skin rendering with actual skins |
| 7 | Full game flow (select → play → result) |
| 9 | Launcher settings GUI |
| 10 | Song database operations |
| 11 | Cross-crate compilation without stubs |
| 12 | Application launches (blank window) |
| 13 | Full game playable |
| 14 | All `stubs.rs` files eliminated or reduced to rendering-only |
| 15 | All non-rendering stubs eliminated, trait-based DI for lifecycle types |
