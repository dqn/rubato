# Porting TODO — Mechanical Line-by-Line Translation

Dependency graph order. Each module is ported only after its dependencies are complete.

## Completed Phases

Phases 1–12, 13a–f, 13f follow-up, 13g, 14, 15a–g, 16a, 16c, 17 — all complete. 936 tests pass. Zero runtime `todo!()`/`unimplemented!()`. See AGENTS.md for details.

## Phase 13f: egui UI (complete)

- [x] Full egui UI integration (launcher views, mod menu) — EguiIntegration in beatoraja-render wraps egui-wgpu 0.31 + wgpu 24 with `forget_lifetime()` for RenderPass. beatoraja-bin has two event loops: LauncherApp (standalone egui config UI) and BeatorajaApp (game + egui overlay). All 10 modmenu sub-menus ported to egui widgets. LauncherUi with 11 tabs.
- [x] Monitor enumeration on non-macOS → winit `ActiveEventLoop::available_monitors()` — cached via `update_monitors_from_winit()` called from both event loops' `resumed()` handlers

## Phase 13f follow-up: egui UI refinement (complete)

- [x] Wire remaining LauncherUi tabs to PlayConfigurationView fields — All 11 tabs now functional: Input (keyboard duration, controller JKOC/analog scratch, mouse scratch), Skin (slot display, CIM toggle), Other (IPFS, HTTP download, clipboard screenshot), IR (multi-slot config with name/userid/password/send mode/import), Stream (enable/notify/max request), OBS (WebSocket enable/host/port/password/recording mode/stop wait). All read/write Config and PlayerConfig fields directly.
- [x] Remove dead legacy `show()` methods in modmenu — Deleted old `show(&mut ImBoolean)` from 9 sub-menus (freq_trainer, judge_trainer, random_trainer, song_manager, download_task, skin_menu, skin_widget_manager, performance_monitor, misc_setting). Cleaned up unused `ImBoolean`/`imgui_renderer` imports where safe.

## Phase 13f follow-up 2: LauncherUi fixes

- [ ] Fix `commit_config()` to also persist PlayerConfig — currently only saves Config via `Config::write()`; Input/IR/Stream tab changes are lost. Java `PlayConfigurationView.commit()` calls both `Config::write()` and `PlayerConfig::write()`
- [ ] Use `IRConfig::get_userid()`/`set_userid()` and `get_password()`/`set_password()` in IR tab — current code reads/writes raw `ir.userid`/`ir.password` fields, bypassing AES encryption logic in `IRConfig::validate()`
- [ ] Add skin browsing UI to Skin tab — currently shows slot paths only; Java `SkinConfigurationView` has skin type selector, header list from filesystem scan, and dynamic customization options (CustomOption/CustomFile/CustomOffset)
- [ ] Add key binding editing grid to Input tab — currently shows controller settings only; Java `InputConfigurationView` has a full key assignment table per mode with keyboard/controller/MIDI columns

## Phase 16b: Golden Master Test Activation (incomplete)

- [ ] Add missing fixtures for modules not yet covered (modmenu, select bar, stream) — deferred until Java exporter updated
- [ ] Reactivate remaining 17 pending test files — blocked on multiple levels:
  - **Fictional crate names:** tests import `bms_config`, `bms_skin`, `bms_audio`, `bms_render` which don't exist; actual crates are `beatoraja_skin`, `beatoraja_audio`, `beatoraja_render`, `beatoraja_types`
  - **Missing modules:** `bms_render::eval`, `bms_render::state_provider::{StaticStateProvider, SkinStateProvider}` not implemented — blocks `render_snapshot.rs`, `compare_eval_test_skins.rs` (10 tests)
  - **Missing audio API:** `bms_audio::decode::load_audio()`, `bms_audio::renderer::f32_to_i16()` not implemented — blocks `compare_audio.rs` (11 tests)
  - **API signature mismatch:** tests assume free functions (`json_loader::load_skin()`), actual API uses struct methods (`JsonSkinLoader.load_skin()`) — blocks `compare_skin.rs` (13 tests)
  - **Type/field divergence:** tests reference `SkinObjectType` (actual: `SkinObject`), `skin.width`/`skin.objects` as pub (actual: private), `skin.scale_x`/`skin.scale_y`/`skin.options`/`skin.custom_events`/`skin.custom_timers` not present
  - **e2e_helpers.rs:** blocks 7 E2E tests (course_e2e, e2e_edge_cases, e2e_judge, exhaustive_e2e, full_pipeline_integration, replay_roundtrip_e2e, timing_boundary_e2e); depends on JudgeManager integration + KeyInputLog import fixes
  - **Fixture generation:** compare_audio, compare_bga_timeline need Java exporter updates
  - **Resolution:** requires Phase 13f (eval/state_provider), Phase 18 (stub removal), then full test rewrite against actual API

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
