# Porting TODO — Mechanical Line-by-Line Translation

Dependency graph order. Each module is ported only after its dependencies are complete.

## Completed Phases

Phases 1–12, 13a–f, 13f follow-up, 13f follow-up 2, 13g, 14, 15a–g, 16a, 16c, 17 — all complete. 936 tests pass. Zero runtime `todo!()`/`unimplemented!()`. See AGENTS.md for details.

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
- [ ] Add missing fixtures for modules not yet covered (modmenu, select bar, stream) — deferred until Java exporter updated
- [ ] Reactivate remaining 15 pending test files — blocked on multiple levels:
  - **JudgeManager::update() is a stub** (primary blocker, 12 of 15 tests): `beatoraja-play/src/judge_manager.rs` `update(&mut self, _mtime: i64)` has empty body with TODO comment "Phase 7+ dependency — requires BMSPlayer, BMSPlayerInputProcessor, AudioDriver". This is the 400+ line Java judge loop. Blocks: `compare_judge.rs`, `compare_judge_manager.rs`, `compare_replay_e2e.rs`, and all 7 E2E tests via `e2e_helpers.rs`
  - **Missing judge API types:** `JudgeConfig` struct, `JUDGE_PG`/`JUDGE_GR`/etc. constants, `model.build_judge_notes()` do not exist. `GaugeType` is `i32` constants (not enum). `GrooveGauge::new()` signature differs from test expectations
  - **Missing rendering API:** `StaticStateProvider`, `SkinStateProvider`, `render_snapshot` module not implemented — blocks `compare_eval_test_skins.rs`, `compare_render_snapshot.rs` (10 tests)
  - **Missing audio API:** `load_audio()`, `f32_to_i16()` not implemented in `beatoraja-audio` — blocks `compare_audio.rs` (11 tests)
  - **Skin loader API mismatch:** tests assume free functions (`json_loader::load_skin()`), actual API uses struct methods (`JsonSkinLoader.load_skin()`) — blocks `compare_skin.rs` (13 tests). Also: `skin.width`/`skin.objects` are private, `skin.scale_x`/`skin.scale_y`/`skin.options` not present
  - **Missing BGA API:** `BgaProcessor` struct not found — blocks `compare_bga_timeline.rs`
  - **Fixture generation:** compare_audio, compare_bga_timeline need Java exporter updates
  - **Resolution:** requires JudgeManager::update() implementation (Phase 18), rendering state providers, audio decode API, then full test rewrite against actual API

## Phase 18: Post-Phase 13 Lifecycle Wiring

Depends on: Phase 13c (rendering pipeline fully connected). Phase 13f (egui UI) is now complete.

- [ ] Replace `MainController` stubs in 8 crates (select, ir, obs, result, decide, external, modmenu, md-processor) with real `beatoraja-core::MainController` — blocked: downstream crates call crate-specific stub APIs not present on real MainController; requires adapter methods or caller updates per crate
- [ ] Replace `PlayerResource` stubs in 6 crates (select, result, decide, external, modmenu, obs) with real `beatoraja-core::PlayerResource` — blocked: same adapter pattern needed; `PlayerResource` holds rendering/audio handles whose types depend on Phase 13 integration
- [ ] Replace `MainState` stubs with real trait impls — blocked: requires per-screen concrete types (PlayState, SelectState, etc.) to implement the `MainState` trait with real rendering callbacks
- [ ] Remove all `stubs.rs` files (target: zero remaining stubs) — blocked: depends on above three stub replacements completing first
- [ ] Remove `rendering_stubs.rs` (all types replaced by wgpu equivalents from Phase 13) — blocked: skin crates still reference rendering stub types; requires full `beatoraja-render` type propagation
- [ ] E2E gameplay flow test: select → decide → play → result screen transitions — blocked: requires all stubs removed and real screen implementations wired
- [ ] Verify: all tests pass, zero clippy warnings, clean `cargo fmt` — blocked: final gate after all above tasks complete

## Remaining Stubs

- **Lifecycle (trait-ified):** MainController/PlayerResource stubs implement `MainControllerAccess`/`PlayerResourceAccess` traits. MainState stubs use `beatoraja-core` `MainState` trait; downstream stubs have crate-specific APIs
- **External libraries:** LibGDX rendering types (Phase 13 rendering stubs remain in skin crates)
