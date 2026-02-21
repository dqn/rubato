# Porting TODO — Mechanical Line-by-Line Translation

Dependency graph order. Each module is ported only after its dependencies are complete.

## Completed Phases (1–12, 14, 15, 17)

| Phase | Description | Scope |
|-------|-------------|-------|
| 1 | Core Foundation (bms.model, bms.table) | 11,178 lines |
| 2 | Format Variants (bmson, osu) | 1,802 lines |
| 3 | Low-level Subsystems (exceptions, system, util, controller, discord, input, audio, mdprocessor) | 12,050 lines |
| 4 | Configuration & Central State (config, root) | 26,290 lines |
| 5 | Pattern & Gameplay (pattern, play, bga) | 17,692 lines |
| 6 | Skin System (base, json, lr2, lua, property) | 15,594 lines |
| 7 | Screen Implementations (select, result, decide) | 11,900 lines |
| 8 | Advanced Features (ir, external, obs, modmenu, stream) | 15,946 lines |
| 9 | Launcher (egui) | 9,210 lines |
| 10 | Remaining Modules (song, controller, system) | 2,726 lines |
| 11 | Integration & Wiring (stub→real cross-crate imports, 30+ getters, circular dep resolution) | — |
| 12 | Binary Entry Point (beatoraja-bin, winit event loop, CLI args) | — |
| 14 | beatoraja-types crate, circular dep resolution, API unification, downstream stub replacement | 15 modules |
| 15a | SongData/SongInformation/IpfsInformation → beatoraja-types | — |
| 15b | SkinType/GrooveGauge/GaugeProperty → beatoraja-types | — |
| 15c | SongDatabaseAccessor/IRConnection/BMSPlayerInputProcessor struct-vs-trait unification | — |
| 15d | MainControllerAccess/PlayerResourceAccess traits + MainStateType extraction | — |
| 15g | TableData/CourseData cascade unification | — |
| 15e | Platform-specific replacements (Twitter→bail, clipboard→arboard, PortAudio→cpal, monitors→CoreGraphics FFI) | — |
| 15f | Final stub cleanup (unused stubs removed across 9 crates, rendering stubs isolated) | — |
| 17 | Independent stub resolution — verified: zero runtime `todo!()`/`unimplemented!()`, 936 tests pass | — |

## Phase 13: External Library Integration (partially complete)

### Completed Sub-phases
- **13a:** rfd file dialogs, LR2 score import, AES crypto, tar.gz (`flate2`+`tar`), 7z (`sevenz-rust`)
- **13b:** OGG/MP3/FLAC decoding (`lewton`/`symphonia`), Kira 0.12 playback, `ebur128` loudness
- **13c:** `beatoraja-render` crate (SpriteBatch, Texture, Pixmap, GpuContext, wgpu surface); `rendering_stubs.rs` → `pub use beatoraja_render::*` (630→15 lines)
- **13d:** LR2 CSV/Play/JSON loaders, property factories, font rendering
- **13e:** mlua Lua VM init, script-backed properties, skin config export

### 13f: egui UI (partial)
- [x] `todo!()` → `log::warn!()` fallbacks across launcher, modmenu, select, result, decide
- [x] `open_url_in_browser` / `open_folder_in_file_manager` → `open` crate
- [ ] Full egui UI integration (launcher views, mod menu) — deferred: requires egui-wgpu render pass integration into the winit event loop (`beatoraja-bin`), egui widget porting for all launcher settings views, and mod menu overlay. Blocked until `beatoraja-render` GpuContext exposes an egui-compatible surface.
- [ ] Monitor enumeration on non-macOS → winit `ActiveEventLoop::available_monitors()` — blocked: requires the egui/winit event loop to be running (Phase 13f egui integration)

### 13g: FFmpeg / Remaining (partial)
- [x] `todo!()` → `log::warn!()` fallbacks across core, types, obs, ir, external, controller
- [x] FFmpeg → `ffmpeg-next` (BGA video decoding) — `beatoraja-skin` with `ffmpeg` feature flag; falls back to `log::warn!()` when disabled
- [x] javax.sound.midi → `midir` (MIDI device input) — `beatoraja-input` with mpsc channel bridge
- [x] PortAudio → Kira audio playback driver — `beatoraja-audio` `PortAudioDriver` backed by Kira `AudioManager`
- [x] BGA `MovieSeekThread` (background video decoding) — `std::thread` + `mpsc` channels; background thread runs FFmpeg decode loop with frame-skip (`fpsd`), command queue (Play/Loop/Stop/Halt), generational restart, and `Arc<Mutex<SharedState>>` for decoded RGBA frame handoff; gated behind `ffmpeg` feature flag on `beatoraja-play`
- [x] Keysound loading pipeline — `PortAudioDriver`/`GdxSoundDriver` `set_model()` loads WAV via `StaticSoundData::from_file()`, `play_note()`/`play_path()` implemented with Kira playback (sound slicing deferred)
- [x] Keysound sound slicing — Kira `StaticSoundData.slice` field for zero-copy sub-sample; `set_model()` collects notes by wav ID with dedup, `play_note_internal()`/`stop_note_internal()`/`set_volume_note_internal()` dispatch to slice handles
- [x] Keysound parallel loading — rayon `par_iter()` for file I/O in `set_model()`, matching Java `parallelStream()`
- [x] AudioCache keysound deduplication — file-level `HashMap<String, FileCacheEntry>` with generational eviction (`ResourcePool.disposeOld()`); cache persists across `set_model()` calls, rayon loads only uncached paths, `evict_old_cache()` ages entries by `song_resource_gen`
- [x] `play_judge()` / `set_additional_key_sound()` — judge sound playback implemented in both GdxSoundDriver and PortAudioDriver with Kira playback, sound caching, and per-judge/timing handle management
- [x] Windows named pipe IPC (`discord-rpc`) — `std::fs::File` + `OpenOptions` opens `\\.\pipe\discord-ipc-0` as regular file; `read_exact`/`write_all` for I/O; no external crate needed; compiles cross-platform, connects only on Windows with Discord running

## Remaining Stubs (Cannot Replace Yet)

- ~~**Rendering:** TextureRegion/Texture in `beatoraja-play`~~ → resolved: `pub use beatoraja_render::Texture` (Phase 14)
- **Lifecycle (trait-ified):** MainController/PlayerResource stubs remain but implement `MainControllerAccess`/`PlayerResourceAccess` traits. MainState stubs use `beatoraja-core` `MainState` trait; downstream stubs have crate-specific APIs
- **External libraries:** LibGDX rendering types (Phase 13), ImGui/egui types (Phase 13)

## Phase 16: Test Coverage Expansion (partially complete)

936 tests across 11 crates. Golden Master: 29/29 pass + 8 reactivated + 1 `#[ignore]` fixed.

### Completed
- **16a:** Unit tests — bms-model(147), beatoraja-core(115), beatoraja-play(157), beatoraja-pattern(169), beatoraja-types(127) = 715 tests
- **16c:** Integration tests — pattern pipeline(4), config round-trip(6), course data(18), score roundtrip(4) = 32 tests

### 16b: Golden Master Test Activation (partial)
- [x] Rewrite golden-master Cargo.toml/lib.rs to use correct workspace crate names and actual bms-model API
- [x] Enable 29 golden-master comparison tests (channel_extended fixed)
- [x] Reactivate 8 pending test files: compare_config(6), compare_database(23), compare_course_data(4), compare_song_information(23), compare_autoplay(23), compare_pattern_modifiers(5), compare_replay(3), compare_score_data_property(1)
- [x] Fix serde rename mismatches, LN duration counting, mainbpm tie-breaking, CourseDataConstraint aliases, TimeLine bounds checking
- [ ] Add missing fixtures for modules not yet covered (modmenu, select bar, stream) — deferred until Java exporter updated
- [ ] Reactivate remaining 17 pending test files — blocked: compare_pattern (make_random private), compare_bga_timeline (BGAProcessor stubbed), Tier 3 tests (e2e_helpers, render snapshots, judge/rule API mismatch)
- [x] Investigate BMS decoder mode detection discrepancy — investigated: no actual discrepancy found; both Java and Rust correctly detect `longnote_types.bms` as BEAT_5K; fixture expectation was outdated

## Phase 18: Post-Phase 13 Lifecycle Wiring

Depends on: Phase 13f (egui UI integration) and Phase 13c (rendering pipeline fully connected).

- [ ] Replace `MainController` stubs in 8 crates (select, ir, obs, result, decide, external, modmenu, md-processor) with real `beatoraja-core::MainController` — blocked: downstream crates call crate-specific stub APIs not present on real MainController; requires adapter methods or caller updates per crate
- [ ] Replace `PlayerResource` stubs in 6 crates (select, result, decide, external, modmenu, obs) with real `beatoraja-core::PlayerResource` — blocked: same adapter pattern needed; `PlayerResource` holds rendering/audio handles whose types depend on Phase 13 integration
- [ ] Replace `MainState` stubs with real trait impls — blocked: requires per-screen concrete types (PlayState, SelectState, etc.) to implement the `MainState` trait with real rendering callbacks
- [ ] Remove all `stubs.rs` files (target: zero remaining stubs) — blocked: depends on above three stub replacements completing first
- [ ] Remove `rendering_stubs.rs` (all types replaced by wgpu equivalents from Phase 13) — blocked: skin crates still reference rendering stub types; requires full `beatoraja-render` type propagation
- [ ] E2E gameplay flow test: select → decide → play → result screen transitions — blocked: requires all stubs removed and real screen implementations wired
- [ ] Verify: all tests pass, zero clippy warnings, clean `cargo fmt` — blocked: final gate after all above tasks complete

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
| 14–15 | All non-rendering stubs eliminated, trait-based DI |
| 16 | Unit tests for core crates, Golden Master tests activated |
| 17 | No `todo!()` panics in non-rendering code paths |
| 18 | All `stubs.rs` eliminated, full E2E gameplay flow |
