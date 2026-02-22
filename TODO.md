# Porting TODO — Mechanical Line-by-Line Translation

Dependency graph order. Each module is ported only after its dependencies are complete.

## Completed Phases

Phases 1–12, 13a–f, 13f follow-up, 13f follow-up 2, 13g, 14, 15a–g, 16a, 16c, 17 — all complete. 1200 tests pass. Zero runtime `todo!()`/`unimplemented!()`. Phase 18a (core judge loop) complete. Phase 18b (rendering state providers) complete. Phase 18c (audio decode API) complete. Phase 18f (e2e test activation) complete. Phase 18g (BRD replay codec) complete. See AGENTS.md for details.

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
- [ ] Reactivate remaining 4 pending test files — blocked on 18b + 18d:
  - **~~Missing rendering API~~** (resolved by 18b): `StaticStateProvider`, `SkinStateProvider`, `render_snapshot`, `eval` modules now implemented. **Still blocked:** `compare_eval_test_skins.rs` requires `test-bms/test-skin/` directory with test skin files (not present); `compare_render_snapshot.rs` requires `skin/ECFN/` directory with ECFN skin files (not present). Both need Java-exported golden master fixtures to be generated
  - **Skin loader API mismatch:** tests assume free functions (`json_loader::load_skin()`), actual API uses struct methods (`JsonSkinLoader.load_skin()`) — blocks `compare_skin.rs` (13 tests). Also: `skin.width`/`skin.objects` are private, `skin.scale_x`/`skin.scale_y`/`skin.options` not present
  - **Missing BGA API:** `BgaProcessor` struct not found — blocks `compare_bga_timeline.rs`
  - **Resolution:** 18b complete (rendering API implemented). Remaining blockers: test skin fixture files (test-bms/test-skin/, skin/ECFN/), 18d (BGA + skin test rewrite)

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

### 18d: BGA and skin test APIs (unblocks 2 Phase 16b tests)

- [ ] Implement `BgaProcessor` — BGA (background animation) timeline processing; translate from Java
- [ ] Rewrite `compare_skin.rs` against actual API — adapt from free functions (`json_loader::load_skin()`) to struct methods (`JsonSkinLoader.load_skin()`), fix private field access (`skin.width` etc.)

### 18e: Stub replacement and cleanup

- [ ] Replace `MainController` stubs in 8 crates (select, ir, obs, result, decide, external, modmenu, md-processor) with real `beatoraja-core::MainController` — blocked: downstream crates call crate-specific stub APIs not present on real MainController; requires adapter methods or caller updates per crate
- [ ] Replace `PlayerResource` stubs in 6 crates (select, result, decide, external, modmenu, obs) with real `beatoraja-core::PlayerResource` — blocked: same adapter pattern needed; `PlayerResource` holds rendering/audio handles whose types depend on Phase 13 integration
- [ ] Replace `MainState` stubs with real trait impls — blocked: requires per-screen concrete types (PlayState, SelectState, etc.) to implement the `MainState` trait with real rendering callbacks
- [ ] Remove all `stubs.rs` files (target: zero remaining stubs) — blocked: depends on above three stub replacements completing first
- [ ] Remove `rendering_stubs.rs` (all types replaced by wgpu equivalents from Phase 13) — blocked: skin crates still reference rendering stub types; requires full `beatoraja-render` type propagation

### 18f: Integration verification (complete — 9 of 9 e2e test files activated)

- [x] Rewrite e2e test files against actual API — all 9 files rewritten and compile-verified: `e2e_judge.rs`, `course_e2e.rs`, `compare_judge.rs`, `exhaustive_e2e.rs`, `e2e_edge_cases.rs`, `timing_boundary_e2e.rs`, `replay_roundtrip_e2e.rs`, `full_pipeline_integration.rs`, `compare_replay_e2e.rs`. Old API names (`BmsDecoder`/`BmsModel`/`GaugeType` enum/`PlayerRule`/`model.total_notes()`/`score.judge_count()`) replaced with actual crate types (`BMSDecoder`/`BMSModel`/`i32` gauge constants/`BMSPlayerRule`/`model.get_total_notes()`/`score.get_judge_count_total()`). Replay tests use JSON serde round-trip
- [x] Activate 9 e2e test files — moved from `tests/pending/` to `tests/`. All 138 new tests pass. Fixed `build_judge_notes()` time ordering bug discovered during activation (was lane-grouped, now sorted by `(time_us, lane)` with pair_index remapping). 1185 total tests pass
- [ ] Activate remaining 4 Phase 16b pending tests — depends on 18b + 18d completing (compare_eval_test_skins, compare_render_snapshot, compare_skin, compare_bga_timeline)
- [ ] E2E gameplay flow test: select → decide → play → result screen transitions — blocked: requires all stubs removed and real screen implementations wired
- [ ] Verify: all tests pass, zero clippy warnings, clean `cargo fmt` — blocked: final gate after all above tasks complete

### 18g: BRD replay file codec (complete)

- [x] Implement `ReplayData::read_brd()` / `write_brd()` — standalone gzip-compressed JSON read/write on `ReplayData` in `beatoraja-types`. `write_brd()` calls `shrink()` before serialization (matching Java `PlayDataAccessor.wrireReplayData()`). `read_brd()` calls `validate()` after deserialization (matching Java `readReplayData()`). Creates parent directories automatically
- [x] Implement `ReplayData::read_brd_course()` / `write_brd_course()` — course variant for `Vec<ReplayData>` arrays. `write_brd_course()` calls `shrink()` on each element
- [x] Refactor `PlayDataAccessor` to delegate to `ReplayData::read_brd`/`write_brd` — removed duplicate gzip/serde logic. `write_replay_data` now takes `&mut ReplayData` (was `&ReplayData`) to support `shrink()` call. Unused imports (`BufReader`, `BufWriter`, `flate2`, `Validatable`) cleaned up
- [x] 5 new unit tests: BRD round-trip, parent dir creation, nonexistent file error, course round-trip, shrink-on-write verification. 14 total replay_data tests pass

### New Issues Found

- [x] `build_judge_notes()` returned notes in lane-grouped order instead of time order — caused `bpm_extreme_timing_structure` and `multi_stop_timing_gaps` tests to fail. Fixed by sorting by `(time_us, lane)` and remapping `pair_index` values
- [ ] Missing test skin fixture directories — `test-bms/test-skin/` and `skin/ECFN/` directories do not exist in the repository. These are needed by pending golden master tests `compare_eval_test_skins.rs` and `compare_render_snapshot.rs`. Need to either add sample skin files or generate fixtures from Java exporter

## Remaining Stubs

- **Lifecycle (trait-ified):** MainController/PlayerResource stubs implement `MainControllerAccess`/`PlayerResourceAccess` traits. MainState stubs use `beatoraja-core` `MainState` trait; downstream stubs have crate-specific APIs
- **External libraries:** LibGDX rendering types (Phase 13 rendering stubs remain in skin crates)
