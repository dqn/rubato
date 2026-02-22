# Porting TODO — Mechanical Line-by-Line Translation

Dependency graph order. Each module is ported only after its dependencies are complete.

## Completed Phases

Phases 1–12, 13a–f, 13f follow-up, 13f follow-up 2, 13g, 14, 15a–g, 16a, 16c, 17 — all complete. 1241 tests pass. Zero runtime `todo!()`/`unimplemented!()`. Phase 18a (core judge loop) complete. Phase 18b (rendering state providers) complete. Phase 18c (audio decode API) complete. Phase 18d (BGA/skin test APIs) complete. Phase 18e-1 (cross-crate stub deduplication) complete. Phase 18e-2 (lifecycle stub replacement) partially complete — obs/external/decide/ir/select/modmenu MainController done (7 of 8), result MainController remaining (blocked on type mismatches); PlayerResource wrapper migration complete for all 6 crates (select/decide/external/obs/modmenu/result). Phase 18e-3 (modmenu skin stub replacement) complete. Phase 18e-4 (PlayDataAccessor stub replacement + IntArray removal) complete. Phase 18e-5 (BMSPlayerMode replacement + dead code removal) complete. Phase 18e-6 (ImGui adapter stubs + Clipboard + freq_trainer replacement) complete. Phase 18e-7 (beatoraja-select stub cleanup) complete. Phase 18f (e2e test activation) complete. Phase 18g (BRD replay codec) complete. See AGENTS.md for details.

## Phase 13f: egui UI (complete)

- [x] Full egui UI integration (launcher views, mod menu) — EguiIntegration in beatoraja-render wraps egui-wgpu 0.31 + wgpu 24 with `forget_lifetime()` for RenderPass. beatoraja-bin has two event loops: LauncherApp (standalone egui config UI) and BeatorajaApp (game + egui overlay). All 10 modmenu sub-menus ported to egui widgets. LauncherUi with 11 tabs.
- [x] Monitor enumeration on non-macOS → winit `ActiveEventLoop::available_monitors()` — cached via `update_monitors_from_winit()` called from both event loops' `resumed()` handlers

## Phase 13f follow-up: egui UI refinement (complete)

- [x] Wire remaining LauncherUi tabs to PlayConfigurationView fields — All 11 tabs now functional: Input (keyboard duration, controller JKOC/analog scratch, mouse scratch), Skin (slot display, CIM toggle), Other (IPFS, HTTP download, clipboard screenshot), IR (multi-slot config with name/userid/password/send mode/import), Stream (enable/notify/max request), OBS (WebSocket enable/host/port/password/recording mode/stop wait). All read/write Config and PlayerConfig fields directly.
- [x] Remove dead legacy `show()` methods in modmenu — Deleted old `show(&mut ImBoolean)` from 9 sub-menus (freq_trainer, judge_trainer, random_trainer, song_manager, download_task, skin_menu, skin_widget_manager, performance_monitor, misc_setting). Cleaned up unused `ImBoolean`/`imgui_renderer` imports where safe.

## Phase 13f follow-up 2: LauncherUi fixes (complete)

- [x] Fix `commit_config()` to also persist PlayerConfig — now calls both `Config::write()` and `PlayerConfig::write(&config.playerpath, &player)` matching Java `PlayConfigurationView.commit()` + `commitPlayer()`
- [x] Use `IRConfig::get_userid()`/`set_userid()` and `get_password()`/`set_password()` in IR tab — added decrypted buffers (`ir_userid_buf`/`ir_password_buf`) for egui text editing; flush via `set_userid`/`set_password` (triggers AES encryption) on IR slot switch and commit. Password field uses `egui::TextEdit::password(true)`
- [x] Add skin browsing UI to Skin tab — integrated `SkinConfigurationView` into `LauncherUi`: skin type ComboBox (19 types), skin header ComboBox (filesystem-scanned skins), dynamic CustomOption/CustomFile/CustomOffset egui widgets, history-aware save/restore on type/header switch, commit persists to `player.skin`/`skin_history`
- [x] ~~Add key binding editing grid to Input tab~~ — NOT NEEDED: Java `InputConfigurationView` is a settings panel (mode, duration, controller table, mouse scratch) which is already fully implemented in Rust. The per-key binding grid is in separate `KeyConfiguration` game-state class, not part of the launcher UI

## Phase 16b: Golden Master Test Activation (partially complete)

- [x] Delete duplicate pending tests — `compare_rule.rs` and `compare_pattern.rs` in `pending/` were duplicates of already-active versions with real imports; deleted
- [ ] Add missing fixtures for modules not yet covered (modmenu, select bar, stream) — Java exporters exist; deferred until Rust-side APIs are implemented
- [x] Activate 9 e2e test files (Phase 18f) — moved from `pending/` to active: `e2e_judge.rs` (20), `course_e2e.rs` (9), `compare_judge.rs` (6), `compare_replay_e2e.rs` (1), `e2e_edge_cases.rs` (11), `exhaustive_e2e.rs` (72), `timing_boundary_e2e.rs` (10), `full_pipeline_integration.rs` (4), `replay_roundtrip_e2e.rs` (5). Total 138 new tests. Fixed `build_judge_notes()` time ordering bug (was lane-grouped, now sorted by `(time_us, lane)` with pair_index remapping).
- [x] Activate `compare_skin.rs` and `compare_bga_timeline.rs` (Phase 18d) — moved from `pending/` to active. 11 new tests (6 skin + 5 BGA)
- [x] Activate `compare_eval_test_skins.rs` — rewritten from JSON-loading to programmatic skin construction. Uses `SkinImage::new_with_single(TextureRegion::new())` + `set_destination_with_int_timer_ops()`. Tests interpolation modes (acc=0,1,2,3), loop variants (loop=0,50,-1), draw conditions matrix (positive/negative boolean IDs). 12 new tests. 1241 total tests pass
- [ ] Reactivate `compare_render_snapshot.rs` — blocked: uses old crate names (bms_config, bms_render, bms_skin), needs full rewrite against current API. Also needs ECFN skin loading pipeline (JSONSkinLoader returns SkinData not Skin) and Lua skin loader (stubbed). Java fixtures exist in `golden-master/fixtures/render_snapshots_java/`

## Phase 18: Post-Phase 13 Lifecycle Wiring

Depends on: Phase 13c (rendering pipeline fully connected). Phase 13f (egui UI) is now complete.

### 18a: Core judge loop implementation (complete — unblocks 12 of 15 Phase 16b tests)

- [x] Implement `JudgeManager::update()` — full 450-line Java judge loop translated to testable Rust API: `update(&mut self, mtime, &[JudgeNote], &[bool], &[i64], &mut GrooveGauge)`. All 4 sections: pass-through, HCN gauge, key press/release, miss POOR + LN end. Internal `NoteJudgeState` tracks per-note state/play_time. `LaneIterState` reimplements Java `Lane` mark/reset/getNote on flat index arrays. `MultiBadCollector` filters simultaneous bad judgments. `update_micro()` records score/combo/ghost/gauge. 24 tests pass.
- [x] Add `JudgeConfig` struct — `JudgeConfig<'a>` with notes, mode, ln_type, judge_rank, judge_window_rate, scratch_judge_window_rate, algorithm, autoplay, judge_property, lane_property. `JudgeManager::from_config()` constructor.
- [x] Add judge constants (`JUDGE_PG`, `JUDGE_GR`, `JUDGE_GD`, `JUDGE_BD`, `JUDGE_PR`, `JUDGE_MS`) — in `bms-model/src/judge_note.rs`
- [x] Add `BMSModel::build_judge_notes()` — in `bms-model/src/judge_note.rs`, builds flat time-ordered array (sorted by `(time_us, lane)`) with LN pair cross-linking via `pair_index`. 8 tests pass.
- [x] Add `JudgeAlgorithm::compare_times()` — variant of `compare()` taking raw time/state values and `&[[i64; 2]]` judge table (used by `update()` where only `JudgeNote` is available)
- [x] Rewrite `e2e_helpers.rs` and `compare_judge_manager.rs` against actual API — `from_config()`, `mode` field, `GrooveGauge::new(&model, i32, &GaugeProperty)`, `BMSDecoder::new().decode(ChartInformation)`, `KeyInputLog::with_data()`. Added `pair_index` bounds checks in `update_micro()` and HCN gauge loop. Fixed `build_judge_notes()` LN pairing (stack-based post-processing). Fixed `from_config()` total_notes to exclude LN end notes for LNTYPE_LONGNOTE (matches Java). `compare_judge_manager.rs` activated (moved out of pending). 993 tests pass.

### 18b: Rendering state providers (complete — unblocks 2 Phase 16b tests)

- [x] Add `get_id()` to property traits — added default `fn get_id(&self) -> i32 { i32::MIN }` to `BooleanProperty`, `IntegerProperty`, `FloatProperty`, `StringProperty` traits; all factory implementations updated to store and return actual IDs
- [x] Add getter methods to skin objects — `SkinImage::get_ref_prop()`/`get_source_count()`/`has_valid_source()`, `SkinNumber::get_ref_prop()`, `SkinSlider::get_ref_prop()`/`get_direction()`, `SkinGraph::get_ref_prop()`/`get_direction()`
- [x] Implement `SkinStateProvider` trait and `StaticStateProvider` — decoupled state interface for golden-master testing; provides timer/boolean/integer/float/string/offset values; boolean negation via negative IDs; `Serialize`/`Deserialize` on `SkinOffset`
- [x] Implement `eval` module in golden-master — pure-function keyframe evaluation replicating `SkinObjectData::prepare_region/prepare_color/prepare_angle` as immutable functions; `resolve_common()`, `compute_rate()`, `compute_region()`, `compute_color()`, `compute_angle()`, `resolve_text_content()`; 6 unit tests
- [x] Implement `render_snapshot` module in golden-master — snapshot infrastructure for comparing rendered skin state against Java fixtures; `capture_render_snapshot()`, `compare_snapshots()`, type-specific detail resolution for all 12 `SkinObject` variants; draw condition and option evaluation; workaround functions for skin-specific quirks; 4 unit tests

### 18c: Audio decode API (complete — unblocks 1 Phase 16b test)

- [x] Implement `load_audio()` in `beatoraja-audio` — `decode::AudioData` struct with f32 samples, delegates to existing `FloatPCM` for resampling/channel conversion. `decode::load_audio(&Path)` uses `PCMLoader` + `FloatPCM::load_pcm()`. Supports WAV (PCM 8/16/24/32-bit, IEEE float, MS-ADPCM), OGG, MP3, FLAC
- [x] Implement `f32_to_i16()` in `beatoraja-audio` — `bms_renderer::f32_to_i16(&[f32]) -> Vec<i16>`, clamp + scale by `i16::MAX`
- [x] Add WAVE_FORMAT_EXTENSIBLE (0xFFFE) support to WAV reader — reads sub-format GUID to extract actual format type (needed for 24-bit WAV test files). Translated from AudioExporter.java
- [x] Activate `compare_audio.rs` golden master test — moved from `pending/` to active, updated crate references (`bms_audio` → `beatoraja_audio`). 11 tests pass (6 decode + 3 resample + 2 channel conversion)

### 18d: BGA and skin test APIs (complete — unblocks 2 Phase 16b tests)

- [x] Implement `BgaProcessor` timeline processing — `BgaTimeline` struct, `from_model(&BMSModel)`, `set_model_timelines()`, `prepare_bga(time_ms)` (line-by-line Java translation), `update(time_us)` public API, `current_bga_id()`/`current_layer_id()` getters. 8 unit tests
- [x] Rewrite `compare_skin.rs` against actual API — rewrote from free functions to `JSONSkinLoader::new().load_header()`/`.load()` struct methods; tests verify `SkinHeaderData`/`SkinData` (intermediate types, not `Skin`). Fixed 3 bugs in json_skin_loader.rs: source_resolution not set, filepath absolutization, offset defaults for non-PLAY types. Created test fixtures: `test_skin.json`, `test_skin_options.json`, `test_skin.lr2skin`. 6 tests (3 JSON header/load/destinations + 2 JSON options + 1 LR2 header). Skipped: Lua (stubbed), ECFN (external files), full Skin snapshots (SkinData→Skin pipeline not connected)
- [x] Rewrite `compare_bga_timeline.rs` against actual API — rewrote from fixture-based golden master to programmatic verification using `BGAProcessor::from_model()` + `update(time_us)` + `current_bga_id()`/`current_layer_id()`. Tests verify BGA/layer state transitions at measure boundaries against `bga_test.bms` (BPM=120). 5 tests. No Java BGA exporter needed

### 18e: Stub replacement and cleanup (partially complete)

#### 18e-1: Cross-crate stub deduplication (complete)

- [x] Centralize `ImGuiNotify` in `beatoraja-types` — added `imgui_notify.rs` with log-backed facade (info/warning/error/success + `_with_dismiss` variants). Replaced 6 duplicate stubs in: beatoraja-ir, beatoraja-stream, beatoraja-obs, beatoraja-select, beatoraja-external, md-processor. All now re-export from `beatoraja_types::imgui_notify::ImGuiNotify`
- [x] Replace `Random`/`LR2Random` stubs in beatoraja-ir — added `beatoraja-pattern` dependency. Replaced stub enum (SCREAMING_CASE) with real `Random` enum (PascalCase). Replaced stub `LR2Random::new(seed)` with real `LR2Random::with_seed(seed)`. Updated `lr2_ghost_data.rs` callers
- [x] Update beatoraja-external `ImGuiNotify::info(msg, duration)` callers — changed 2-arg `info()` to `info_with_dismiss()` matching real modmenu API. 3 call sites updated (screen_shot_twitter_exporter, screen_shot_file_exporter)
- [x] Add `beatoraja-types` dependency to beatoraja-stream — was the only stub-holding crate without it

#### 18e-2: Lifecycle stub replacement (partially complete)

##### MainController stubs — completed (7 of 8 crates)

- [x] beatoraja-obs: Removed `MainControllerRef` entirely — added `state_type() -> Option<MainStateType>` to `MainState` trait (beatoraja-core), replaced `MainControllerRef::get_state_type(state)` with `state.state_type()` in `obs_listener.rs`. Zero remaining MainController stub code
- [x] beatoraja-external: Replaced `MainController` struct + `MainControllerAccess` impl with `NullMainController` re-export from beatoraja-types. `MainState.main` field type changed to `NullMainController`. No code accesses `state.main` so change is safe
- [x] beatoraja-decide: Removed unused `MainControllerAccess` trait impl from `MainControllerRef`. Struct retained with 3 methods (`change_state`, `get_input_processor`, `get_audio_processor`) that are actively called by `MusicDecide`
- [x] beatoraja-ir: No MainController stub exists — nothing to do
- [x] beatoraja-select: Removed `MainController` struct + 6 dead methods, `DefaultMainState`, `PlayerResource` empty struct. `MainState` trait simplified to empty marker trait (`get_main()` removed). `RandomCourseData::lottery_song_datas` and `ContextMenuBar::fill_missing_charts` updated to remove `&MainController` parameter. Zero call sites existed

##### MainController stubs — remaining (2 crates)
- [ ] beatoraja-result: 6 methods actively used in music_result.rs/course_result.rs (get_play_data_accessor, get_input_processor, get_ir_status, save_last_recording, ir_send_status_mut, ir_send_status). 3 unused methods pruned (get_config, get_player_config, change_state). PlayDataAccessor stub replaced (Phase 18e-4). Remaining blockers: BMSPlayerInputProcessor `&mut` vs `&`, `ir_send_status_mut()` doesn't exist on real MainController, IRConnection not implemented
- [x] beatoraja-modmenu: Removed dead code (`get_current_state()`, `load_new_profile()`, `DefaultMainState`). Replaced stub `PlayerConfig` with real `beatoraja_types::PlayerConfig`. Replaced stub `SkinConfig`/`SkinConfigProperty`/`SkinConfigOption`/`SkinConfigFilePath`/`SkinConfigOffset`/`SkinConfigDefault` with real types (`SkinConfig`/`SkinProperty`/`SkinOption`/`SkinFilePath`/`SkinOffset`). Updated ~15 call sites in skin_menu.rs for `Option<>` handling (`.iter().flatten()`, `is_some_and()`, `Some()` wrapping). Updated misc_setting_menu.rs (`get_play_config(mode.clone())`, `read_all_player_id()` free function). MainController stub retained with 3 methods (get_config, get_player_config, save_config) — needed until real MainController exists. Remaining modmenu stubs: Skin, SkinObject, MainState trait, MainController (see Phase 18e-3 for skin stub replacement)
- [x] md-processor: `MainControllerRef` trait reclassified as **intentional adapter pattern** — `update_song()` is called via `Arc<dyn MainControllerRef>` in `HttpDownloadProcessor::execute_download_task()`. `HttpDownloadProcessor::new()` is never instantiated yet, but architecture is sound (minimal 1-method trait decouples md-processor from full MainController). Deferred until HttpDownloadProcessor activation

##### PlayerResource stubs — completed (all 6 crates)

- [x] beatoraja-select: Removed empty `pub struct PlayerResource;` — zero usage (deleted alongside MainController)
- [x] beatoraja-decide: Replaced `PlayerResourceRef` + `PlayerConfigRef` + 29-method `PlayerResourceAccess` impl with `Box<dyn PlayerResourceAccess>` from beatoraja-types. `MusicDecide::resource` now trait-based. `NullPlayerResource` re-exported for default construction
- [x] beatoraja-obs/beatoraja-modmenu have no PlayerResource stubs
- [x] beatoraja-external: Replaced concrete `PlayerResource` struct (5 fields, 5 methods, 29-method `PlayerResourceAccess` impl) with `Box<dyn PlayerResourceAccess>` wrapper + `original_mode: Mode` (from `bms_model::mode::Mode`). Stub `Mode` struct replaced with real enum. Callers updated to handle `Option<>` returns (`get_songdata()`, `get_replay_data()`). `get_original_mode()` kept as crate-local method (not on trait). 1241 tests pass
- [x] beatoraja-result: Replaced monolithic PlayerResource stub with `Box<dyn PlayerResourceAccess>` wrapper + crate-local fields (`BMSModel`, `BMSPlayerMode`, `RankingData`, `course_bms_models`). All trait-compatible methods delegate to `self.inner`. Callers updated for API changes: `get_replay_data()` → `Option<&ReplayData>` (music_result.rs), `get_songdata()` → `Option<&SongData>` (music_result.rs), `get_course_data()` → `Option<&CourseData>` (course_result.rs), `add_course_replay()` → by-value (music_result.rs), `get_course_score_data_mut()` → get/clone/set pattern (music_result.rs). Removed unused methods: `get_replay_data_mut()`, `reload_bms_file()`. Unused `PlayerResource` import removed from abstract_result.rs. 1241 tests pass

#### 18e-3: Modmenu skin stub replacement (complete)
#### 18e-4: PlayDataAccessor stub replacement + IntArray removal (complete)
#### 18e-5: BMSPlayerMode replacement + dead code removal (complete)

- [x] Replace BMSPlayerMode/BMSPlayerModeType stubs in beatoraja-result with `pub use beatoraja_core::bms_player_mode::{BMSPlayerMode, Mode as BMSPlayerModeType}` — real type has `mode: Mode` + `id: i32`; alias avoids naming conflict with `bms_model::mode::Mode`. All callers use `== BMSPlayerModeType::Play` which maps to `== Mode::Play`. `PlayerResource::default()` updated to use `BMSPlayerMode::new()`
- [x] Remove EventType enum stub from beatoraja-result — dead code only referenced in commented-out `execute_event()` calls. Removed from imports in abstract_result.rs, music_result.rs, course_result.rs
- [x] Delete unused InputProcessor, Lwjgl3ControllerManager, Controller stubs from beatoraja-modmenu — imported in imgui_renderer.rs but never used. ~70 lines removed

#### 18e-6: ImGui adapter stubs + Clipboard + freq_trainer replacement (complete)

- [x] Replace ImBoolean/ImInt/ImFloat stubs with plain `bool`/`i32`/`f32` in `Mutex<T>` — removed ~60 lines of wrapper code from stubs.rs. Updated 7 files: imgui_renderer.rs (10 statics), skin_menu.rs (2 statics), skin_widget_manager.rs (6 statics), misc_setting_menu.rs (15 statics), song_manager_menu.rs (1 static), random_trainer_menu.rs (3 statics), judge_trainer_menu.rs (2 statics). All `.get()`→`*guard`, `.set(val)`→`*guard = val`, `.value`→`*guard`, `ImInt::new(val)`→`val`
- [x] Replace Clipboard stubs with real `arboard` calls — beatoraja-modmenu skin_widget_manager.rs and beatoraja-select context_menu_bar.rs now use `arboard::Clipboard::new()` + `set_text()` directly. Added `arboard` dependency to both crates. Removed Clipboard stubs from both stubs.rs files
- [x] Replace freq_trainer stubs with real re-exports from beatoraja-modmenu — `is_freq_trainer_enabled()` and `is_freq_negative()` free function stubs in beatoraja-result replaced with `pub use beatoraja_modmenu::freq_trainer_menu::FreqTrainerMenu`. Added beatoraja-modmenu dependency to beatoraja-result (no circular dep). Updated 3 call sites in music_result.rs to use `FreqTrainerMenu::is_freq_trainer_enabled()` / `FreqTrainerMenu::is_freq_negative()`

#### 18e-7: beatoraja-select stub cleanup (complete)

- [x] Replace BMSPlayerMode/BMSPlayerModeType stubs in beatoraja-select with `pub use beatoraja_core::bms_player_mode::{BMSPlayerMode, Mode as BMSPlayerModeType}` — same pattern as beatoraja-result (Phase 18e-5). Callers only use BMSPlayerMode as a type (field types and method parameters), no variant access. ~36 lines of stub code → 3 lines of re-export
- [x] Replace PlayerInformation stub in beatoraja-select with `pub use beatoraja_core::player_information::PlayerInformation` — added `get_name() -> &str` method to real PlayerInformation (returns `self.name.as_deref().unwrap_or("")`). Caller in music_selector.rs:123 (`r.get_name()`) works unchanged. ~13 lines removed
- [x] Remove dead Pair<A,B> stub from beatoraja-select — struct defined but never used anywhere in beatoraja-select. ~28 lines removed. Real Pair exists in beatoraja-common but was unused

- [x] Replace PlayDataAccessor stub in beatoraja-result with `pub use beatoraja_core::play_data_accessor::PlayDataAccessor` — removed ~105 lines of no-op stub code
- [x] Add model-based convenience methods to PlayDataAccessor — `read_score_data_model`, `write_score_data_model`, `exists_replay_data_model`, `write_replay_data_model`, `delete_score_data_model` that extract hash from BMSModel and delegate to hash-based methods
- [x] Add course methods to PlayDataAccessor — `read_score_data_course`, `write_score_data_course`, `exists_replay_data_course`, `read_replay_data_course`, `write_replay_data_course` with constraint-based suffix and first-10-chars hash concatenation
- [x] Add `PlayDataAccessor::null()` constructor — all Option fields = None, for use in stubs returning `&PlayDataAccessor`
- [x] Implement `compute_constraint_values` — was TODO returning `(0, 0, 0)`, now properly maps `CourseDataConstraint` variants to `(hispeed, judge, gauge)` tuple
- [x] Fix `write_score_data_for_course` parameter type — was `&[CourseData]`, corrected to `&[CourseDataConstraint]`
- [x] Replace IntArray stub with `Vec<i32>` in skin_gauge_graph_object.rs — `.items.last()` → `.last()`, `.add()` → `.push()`, `.contains(x)` → `.contains(&x)`
- [x] Update MainController.get_play_data_accessor() to use `Box::leak(Box::new(PlayDataAccessor::null()))` — OnceLock not usable because rusqlite::Connection isn't Sync
- [x] Update music_result.rs callers — `write_replay_data` → `write_replay_data_model`, `read_score_data` → `read_score_data_model`, `write_score_data` → `write_score_data_model`
- [x] Update course_result.rs callers — `write_replay_data_course` now clones replays for `&mut` access

- [x] Replace SkinHeader + inner types (CustomOption, CustomFile, CustomOffset, CustomCategory, CustomCategoryItem) with re-exports from beatoraja-skin::skin_header — ~170 lines of stubs removed from stubs.rs
- [x] Replace TYPE_LR2SKIN, OPTION_RANDOM_VALUE constants with re-exports from beatoraja-skin
- [x] Replace JSONSkinLoader, LR2SkinHeaderLoader, LuaSkinLoader stubs with re-exports from real beatoraja-skin loaders
- [x] Remove SkinLoader stub (only used in commented-out code)
- [x] Add conversion helpers: `skin_header_from_json_data()` (SkinHeaderData → SkinHeader) and `skin_header_from_lr2_data()` (LR2SkinHeaderData → SkinHeader) in skin_menu.rs — LR2 module defines separate CustomOption/CustomFile/CustomOffset types requiring explicit conversion
- [x] Adapt all callers for Option-wrapped getters (get_name → Option<&str>, get_path → Option<&PathBuf>, get_skin_type → Option<&SkinType>)
- [x] Remove Debug derive from Skin stub (real SkinHeader doesn't implement Debug)
- Remaining modmenu stubs (out of scope): Skin/SkinObject/SkinObjectDestination/Rectangle (incompatible enum), MainState trait, MainController (3 methods), MusicSelector/Bar/SongBar

##### Other stubs — remaining

- [ ] Replace `MainState` stubs with real trait impls — blocked: requires per-screen concrete types (PlayState, SelectState, etc.) to implement the `MainState` trait with real rendering callbacks. Real `MainState` trait exists in beatoraja-core with full lifecycle API
- [ ] Remove all `stubs.rs` files (target: zero remaining stubs) — blocked: depends on above stub replacements completing first
- [x] ~~Remove `rendering_stubs.rs`~~ — resolved: `beatoraja-skin/src/rendering_stubs.rs` already re-exports real types from `beatoraja-render` (Texture, Color, Rectangle, Pixmap, SpriteBatch, etc.). No longer contains stubs. Downstream crates (select, result) use real types via this re-export chain

### 18f: Integration verification (complete — 9 of 9 e2e test files activated)

- [x] Rewrite e2e test files against actual API — all 9 files rewritten and compile-verified: `e2e_judge.rs`, `course_e2e.rs`, `compare_judge.rs`, `exhaustive_e2e.rs`, `e2e_edge_cases.rs`, `timing_boundary_e2e.rs`, `replay_roundtrip_e2e.rs`, `full_pipeline_integration.rs`, `compare_replay_e2e.rs`. Old API names (`BmsDecoder`/`BmsModel`/`GaugeType` enum/`PlayerRule`/`model.total_notes()`/`score.judge_count()`) replaced with actual crate types (`BMSDecoder`/`BMSModel`/`i32` gauge constants/`BMSPlayerRule`/`model.get_total_notes()`/`score.get_judge_count_total()`). Replay tests use JSON serde round-trip
- [x] Activate 9 e2e test files — moved from `tests/pending/` to `tests/`. All 138 new tests pass. Fixed `build_judge_notes()` time ordering bug discovered during activation (was lane-grouped, now sorted by `(time_us, lane)` with pair_index remapping). 1185 total tests pass
- [x] Activate `compare_skin.rs` and `compare_bga_timeline.rs` (Phase 18d) — 11 new tests
- [x] Activate `compare_eval_test_skins.rs` — rewritten with programmatic skin construction (12 new tests)
- [ ] Activate `compare_render_snapshot.rs` — blocked: old crate names, SkinData→Skin pipeline gap, Lua loader stubbed
- [ ] E2E gameplay flow test: select → decide → play → result screen transitions — blocked: requires all stubs removed and real screen implementations wired
- [ ] Verify: all tests pass, zero clippy warnings, clean `cargo fmt` — blocked: final gate after all above tasks complete

### 18g: BRD replay file codec (complete)

- [x] Implement `ReplayData::read_brd()` / `write_brd()` — standalone gzip-compressed JSON read/write on `ReplayData` in `beatoraja-types`. `write_brd()` calls `shrink()` before serialization (matching Java `PlayDataAccessor.wrireReplayData()`). `read_brd()` calls `validate()` after deserialization (matching Java `readReplayData()`). Creates parent directories automatically
- [x] Implement `ReplayData::read_brd_course()` / `write_brd_course()` — course variant for `Vec<ReplayData>` arrays. `write_brd_course()` calls `shrink()` on each element
- [x] Refactor `PlayDataAccessor` to delegate to `ReplayData::read_brd`/`write_brd` — removed duplicate gzip/serde logic. `write_replay_data` now takes `&mut ReplayData` (was `&ReplayData`) to support `shrink()` call. Unused imports (`BufReader`, `BufWriter`, `flate2`, `Validatable`) cleaned up
- [x] 5 new unit tests: BRD round-trip, parent dir creation, nonexistent file error, course round-trip, shrink-on-write verification. 14 total replay_data tests pass

### New Issues Found

- [x] `build_judge_notes()` returned notes in lane-grouped order instead of time order — caused `bpm_extreme_timing_structure` and `multi_stop_timing_gaps` tests to fail. Fixed by sorting by `(time_us, lane)` and remapping `pair_index` values
- [x] ~~Missing test skin fixture directory~~ — resolved: `compare_eval_test_skins.rs` rewritten to construct skins programmatically, no fixture files needed
- [ ] JSONSkinLoader returns `SkinData` (intermediate), not `Skin` — full loading pipeline (SkinData→Skin) not connected. `load_skin_object_for_type()` returns None for all screen-specific types. Full Skin snapshot tests deferred until pipeline is wired
- [ ] LuaSkinLoader is completely stubbed — `load_header()` and `load_skin()` return None. Lua skin tests skipped
- [ ] json_skin_loader bug fixes applied during Phase 18d — (1) `source_resolution` was not set from JSON w/h fields, (2) custom file paths were incorrectly absolutized with parent dir, (3) offset defaults were applied to non-PLAY skin types (MusicSelect, Decide, etc.)
- [x] Dead pending source files in golden-master — deleted `src/pending/skin_fixtures.rs` and `src/pending/render_snapshot.rs` (outdated copies using old crate names). Removed empty `src/pending/` directory
- [ ] `compare_render_snapshot.rs` more blocked than expected — previously marked as "unblocked" but uses old crate names throughout (bms_config, bms_render, bms_skin), needs SkinData→Skin pipeline for ECFN loading, and Lua loader is stubbed. Requires full API rewrite + loading pipeline before activation
- [x] md-processor `MainControllerRef` reclassified as valid adapter pattern — `update_song()` is called via `Arc<dyn MainControllerRef>` in `HttpDownloadProcessor`. Intentional adapter trait, not a broken stub
- [x] `PlayerResourceAccess` trait expanded with mutable getters — added `get_score_data_mut`, `get_course_replay_mut`, `get_course_gauge_mut` to trait. `NullPlayerResource` changed from unit struct to field-bearing struct with `Default` derive. beatoraja-core `PlayerResource` impl updated. Stub type mismatches resolved (Phase 18e-2 type conversion). Remaining gap: types not on trait (`BMSModel`, `RankingData`, `BMSPlayerMode`) — need wrapper struct pattern
- [x] beatoraja-result MainController: pruned 3 unused methods (`get_config`, `get_player_config`, `change_state`). 6 actively used methods remain
- [x] beatoraja-result stub type conversion complete — `FloatArray` → `Vec<f32>`, `GdxArray<T>` → `Vec<T>`, `GrooveGaugeStub` → real `GrooveGauge` (from beatoraja-types). Callers in music_result.rs, course_result.rs, skin_gauge_graph_object.rs updated. Stub methods `get_gauge()`, `get_groove_gauge()`, `get_course_gauge()`, `get_course_gauge_mut()`, `add_course_gauge()` now match trait signatures. `IntArray` retained (only used in skin_gauge_graph_object.rs)
- [x] beatoraja-result PlayerResource wrapper complete — replaced monolithic stub with `Box<dyn PlayerResourceAccess>` wrapper + crate-local fields for non-trait types (BMSModel, BMSPlayerMode, RankingData, course_bms_models). Removed `get_replay_data_mut()`/`reload_bms_file()` (unused). Converted `get_course_score_data_mut()` to get/clone/set pattern. Callers updated for `Option<>` returns on `get_songdata()`, `get_replay_data()`, `get_course_data()`, and by-value `add_course_replay()`
- [x] `write_score_data_for_course` had wrong parameter type — accepted `&[CourseData]` but should have been `&[CourseDataConstraint]` (matches Java `PlayDataAccessor.writeScoreData` course variant). Fixed in Phase 18e-4
- [x] `compute_constraint_values` was TODO stub — returned `(0, 0, 0)` instead of mapping `CourseDataConstraint` enum variants to `(hispeed, judge, gauge)`. Implemented in Phase 18e-4
- [x] PlayDataAccessor model-based convenience methods missing — real implementation used hash-based signatures (`sha256: &str`) but callers in beatoraja-result used BMSModel-based signatures. Added model-based wrapper methods that extract hash and delegate. Fixed in Phase 18e-4

## Remaining Stubs

- **Lifecycle (mostly resolved):** MainController stubs removed from obs/external/ir/select/modmenu (Phase 18e-2). md-processor reclassified as intentional adapter pattern. modmenu: dead code removed, skin config types replaced with real beatoraja-types, skin stubs replaced with real beatoraja-skin types (Phase 18e-3); 3-method MainController stub retained for lifecycle. Remaining MainController: result (6 methods actively used, 3 unused pruned, PlayDataAccessor resolved in 18e-4, remaining blockers: BMSPlayerInputProcessor/IRConnection). **PlayerResource wrapper migration complete for all 6 crates** — select/decide/external use `Box<dyn PlayerResourceAccess>`, result uses wrapper struct with crate-local fields for non-trait types. MainState stubs require per-screen concrete implementations
- **beatoraja-select remaining stubs:** MainState trait (empty marker), RandomCourseData/RandomStageData, RankingData/RankingDataCache, BMSPlayerInputProcessor/ControlKeys/KeyCommand, SkinText/SkinNumber/SkinImage/SkinObject/SkinObjectRenderer, EventType, AudioDriver, TimerState, NullSongDatabaseAccessor, SongManagerMenu, DownloadTask/DownloadTaskState/DownloadTaskStatus. BMSPlayerMode/PlayerInformation replaced (Phase 18e-7). Dead Pair stub removed (Phase 18e-7). **Blocked:** SongManagerMenu (circular dep modmenu→select), rendering stubs (rendering pipeline), BMSPlayerInputProcessor (input system), RandomCourseData (database queries)
- **beatoraja-modmenu remaining stubs:** Skin/SkinObject/SkinObjectDestination/Rectangle (real SkinObject is incompatible enum), MainState trait, MainController (3 methods), MusicSelector/Bar/SongBar (select screen types). ImBoolean/ImInt/ImFloat replaced with plain primitives (Phase 18e-6). Clipboard replaced with arboard (Phase 18e-6)
- **beatoraja-pattern remaining stubs:** RandomTrainer/RandomHistoryEntry — **blocked:** circular dep (beatoraja-modmenu depends on beatoraja-pattern). Real impl exists in beatoraja-modmenu::random_trainer
- **Rendering re-exports:** `rendering_stubs.rs` in beatoraja-skin now re-exports real beatoraja-render types (resolved, not stubs)
- **beatoraja-result remaining stubs:** MainController (6 methods, PlayDataAccessor resolved), BMSPlayerInputProcessor, IRStatus/IRSendStatusMain, SkinObjectData. freq_trainer stubs replaced with re-exports from beatoraja-modmenu (Phase 18e-6). BMSPlayerMode/BMSPlayerModeType replaced with real types (Phase 18e-5), EventType removed (dead code). PlayerResource wrapper complete; non-trait field types (BMSModel, RankingData) stored locally, BMSPlayerMode now uses real type
