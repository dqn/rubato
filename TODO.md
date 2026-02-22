# Porting TODO — Mechanical Line-by-Line Translation

Dependency graph order. Each module is ported only after its dependencies are complete.

## Completed Phases

Phases 1–12, 13a–f, 13f follow-up, 13f follow-up 2, 13g, 14, 15a–g, 16a, 16c, 17 — all complete. 993 tests pass. Zero runtime `todo!()`/`unimplemented!()`. Phase 18a (core judge loop) complete. See AGENTS.md for details.

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
- [ ] Reactivate remaining 14 pending test files — blocked on multiple levels:
  - ~~**JudgeManager::update() is a stub**~~ → resolved: full judge loop implemented in Phase 18a. New testable API: `update(&mut self, mtime, &[JudgeNote], &[bool], &[i64], &mut GrooveGauge)`
  - ~~**Missing judge API types**~~ → resolved: `JudgeConfig`, `JUDGE_PG`/`JUDGE_GR`/etc. constants, `build_judge_notes()` all implemented in Phase 18a
  - ~~**e2e_helpers.rs rewrite still needed**~~ → resolved: rewritten against actual API (Phase 18a complete). `e2e_helpers.rs` activated in lib.rs. `compare_judge_manager.rs` moved out of pending/
  - **Missing rendering API:** `StaticStateProvider`, `SkinStateProvider`, `render_snapshot` module not implemented — blocks `compare_eval_test_skins.rs`, `compare_render_snapshot.rs` (10 tests)
  - **Missing audio API:** `load_audio()`, `f32_to_i16()` not implemented in `beatoraja-audio` — blocks `compare_audio.rs` (11 tests)
  - **Skin loader API mismatch:** tests assume free functions (`json_loader::load_skin()`), actual API uses struct methods (`JsonSkinLoader.load_skin()`) — blocks `compare_skin.rs` (13 tests). Also: `skin.width`/`skin.objects` are private, `skin.scale_x`/`skin.scale_y`/`skin.options` not present
  - **Missing BGA API:** `BgaProcessor` struct not found — blocks `compare_bga_timeline.rs`
  - **Fixture availability:** compare_audio, compare_bga_timeline fixtures and Java exporters already exist; blocker is Rust-side API (`load_audio()`, `BgaProcessor`)
  - **Resolution:** Phase 18a (judge loop) complete. Remaining blockers: 18b (rendering state providers), 18c (audio decode API), 18d (BGA + skin test rewrite), then 18f (test activation)

## Phase 18: Post-Phase 13 Lifecycle Wiring

Depends on: Phase 13c (rendering pipeline fully connected). Phase 13f (egui UI) is now complete.

### 18a: Core judge loop implementation (complete — unblocks 12 of 15 Phase 16b tests)

- [x] Implement `JudgeManager::update()` — full 450-line Java judge loop translated to testable Rust API: `update(&mut self, mtime, &[JudgeNote], &[bool], &[i64], &mut GrooveGauge)`. All 4 sections: pass-through, HCN gauge, key press/release, miss POOR + LN end. Internal `NoteJudgeState` tracks per-note state/play_time. `LaneIterState` reimplements Java `Lane` mark/reset/getNote on flat index arrays. `MultiBadCollector` filters simultaneous bad judgments. `update_micro()` records score/combo/ghost/gauge. 24 tests pass.
- [x] Add `JudgeConfig` struct — `JudgeConfig<'a>` with notes, mode, ln_type, judge_rank, judge_window_rate, scratch_judge_window_rate, algorithm, autoplay, judge_property, lane_property. `JudgeManager::from_config()` constructor.
- [x] Add judge constants (`JUDGE_PG`, `JUDGE_GR`, `JUDGE_GD`, `JUDGE_BD`, `JUDGE_PR`, `JUDGE_MS`) — in `bms-model/src/judge_note.rs`
- [x] Add `BMSModel::build_judge_notes()` — in `bms-model/src/judge_note.rs`, builds flat lane-grouped array with LN pair cross-linking via `pair_index`. 7 tests pass.
- [x] Add `JudgeAlgorithm::compare_times()` — variant of `compare()` taking raw time/state values and `&[[i64; 2]]` judge table (used by `update()` where only `JudgeNote` is available)
- [x] Rewrite `e2e_helpers.rs` and `compare_judge_manager.rs` against actual API — `from_config()`, `mode` field, `GrooveGauge::new(&model, i32, &GaugeProperty)`, `BMSDecoder::new().decode(ChartInformation)`, `KeyInputLog::with_data()`. Added `pair_index` bounds checks in `update_micro()` and HCN gauge loop. Fixed `build_judge_notes()` LN pairing (stack-based post-processing). Fixed `from_config()` total_notes to exclude LN end notes for LNTYPE_LONGNOTE (matches Java). `compare_judge_manager.rs` activated (moved out of pending). 993 tests pass.

### 18b: Rendering state providers (unblocks 2 Phase 16b tests)

- [ ] Implement `StaticStateProvider` and `SkinStateProvider` — provide timer/number/flag values to skin evaluation engine
- [ ] Implement `render_snapshot` module in golden-master — snapshot infrastructure for comparing rendered skin state against Java fixtures

### 18c: Audio decode API (unblocks 1 Phase 16b test)

- [ ] Implement `load_audio()` in `beatoraja-audio` — decode audio files (WAV/OGG/MP3) and return sample data; Kira handles playback but tests need raw sample comparison
- [ ] Implement `f32_to_i16()` in `beatoraja-audio` — sample format conversion for golden master comparison

### 18d: BGA and skin test APIs (unblocks 2 Phase 16b tests)

- [ ] Implement `BgaProcessor` — BGA (background animation) timeline processing; translate from Java
- [ ] Rewrite `compare_skin.rs` against actual API — adapt from free functions (`json_loader::load_skin()`) to struct methods (`JsonSkinLoader.load_skin()`), fix private field access (`skin.width` etc.)

### 18e: Stub replacement and cleanup

- [ ] Replace `MainController` stubs in 8 crates (select, ir, obs, result, decide, external, modmenu, md-processor) with real `beatoraja-core::MainController` — blocked: downstream crates call crate-specific stub APIs not present on real MainController; requires adapter methods or caller updates per crate
- [ ] Replace `PlayerResource` stubs in 6 crates (select, result, decide, external, modmenu, obs) with real `beatoraja-core::PlayerResource` — blocked: same adapter pattern needed; `PlayerResource` holds rendering/audio handles whose types depend on Phase 13 integration
- [ ] Replace `MainState` stubs with real trait impls — blocked: requires per-screen concrete types (PlayState, SelectState, etc.) to implement the `MainState` trait with real rendering callbacks
- [ ] Remove all `stubs.rs` files (target: zero remaining stubs) — blocked: depends on above three stub replacements completing first
- [ ] Remove `rendering_stubs.rs` (all types replaced by wgpu equivalents from Phase 13) — blocked: skin crates still reference rendering stub types; requires full `beatoraja-render` type propagation

### 18f: Integration verification

- [ ] Rewrite e2e test files against actual API — `e2e_judge.rs`, `course_e2e.rs`, `compare_judge.rs`, `exhaustive_e2e.rs`, `e2e_edge_cases.rs`, `timing_boundary_e2e.rs`, `replay_roundtrip_e2e.rs`, `full_pipeline_integration.rs`, `compare_replay_e2e.rs` all use old API names (`BmsDecoder`/`BmsModel`/`GaugeType` enum/`PlayerRule`/`model.total_notes()`) that don't match actual crate types (`BMSDecoder`/`BMSModel`/`i32` gauge constants/`BMSPlayerRule`). Same pattern as compare_judge_manager.rs rewrite
- [ ] Activate remaining 14 Phase 16b pending tests — depends on 18a–18d completing + e2e test API rewrites
- [ ] E2E gameplay flow test: select → decide → play → result screen transitions — blocked: requires all stubs removed and real screen implementations wired
- [ ] Verify: all tests pass, zero clippy warnings, clean `cargo fmt` — blocked: final gate after all above tasks complete

## Remaining Stubs

- **Lifecycle (trait-ified):** MainController/PlayerResource stubs implement `MainControllerAccess`/`PlayerResourceAccess` traits. MainState stubs use `beatoraja-core` `MainState` trait; downstream stubs have crate-specific APIs
- **External libraries:** LibGDX rendering types (Phase 13 rendering stubs remain in skin crates)
