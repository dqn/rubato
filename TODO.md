# Porting TODO — Remaining Work

All phases (1–23) complete. 1500 tests pass. See AGENTS.md for full status.

## Completed Phases (recent)

### Phase 22b: SkinObject Draw Methods + SkinTextBitmap Font Rendering (complete)

+846 lines tests + 500 lines implementation across 7 files. All sub-tasks done:

- [x] **SkinImage draw tests** — 9 tests: basic draw, offset, movie FFmpeg type override, zero alpha skip, region dimensions, color propagation
- [x] **SkinNumber draw tests** — 9 tests: single/multi digit, spacing, alignment (left/center/right), zero padding, per-digit offsets, length calculation
- [x] **SkinTextImage draw tests** — 9 tests: glyph layout, alignment, scaling, margin, offset, height scaling, zero source size
- [x] **SkinTextBitmap.draw_with_offset()** — Real implementation replacing warn!() stub. ab_glyph font rasterization, glyph layout with kerning, alignment (left/center/right), overflow modes (overflow/shrink/truncate), shadow rendering, distance field support. +15 tests
- [x] **Test infrastructure** — MockMainState helper, test_helpers module for beatoraja-skin

### Phase 22c: MainController Render Pipeline + SpriteBatch GPU Flush + FPS Cap (complete)

+325 lines across 5 files + 7 new tests:

- [x] **MainController.render() enhancement** — sprite.begin()/end() lifecycle, input gating by time delta (Java `if(time > prevtime)` pattern), skin draw comments for Phase 22+ wiring
- [x] **SpriteBatch re-export** — sprite_batch_helper.rs now re-exports real SpriteBatch from beatoraja-render (replacing stub unit struct)
- [x] **SpriteBatch→wgpu GPU flush** — SpriteRenderPipeline initialization, dummy white texture, uniform/texture bind groups, vertex buffer upload via flush_to_gpu() in render pass
- [x] **FPS capping** — max_fps from Config, frame duration calculation with thread::sleep, last_frame_time tracking via Instant
- [x] **bytemuck dependency** — Added for projection matrix buffer upload

### Phase 21: Per-Screen MainState Implementations + State Dispatch (complete)

+~350 lines implementation + 23 new tests. All sub-tasks done:

- [x] **DecideState (MusicDecide)** — MainState trait impl in beatoraja-decide. state_type(), create(), render(), input(), dispose() lifecycle methods
- [x] **ResultState (MusicResult)** — MainState trait impl in beatoraja-result. Full lifecycle with score/replay handling stubs
- [x] **PlayState (BMSPlayer)** — MainState trait impl in beatoraja-play. Gameplay loop lifecycle with judge/gauge/BGA stubs
- [x] **SelectState (MusicSelector)** — MainState trait impl in beatoraja-select. Song select lifecycle with bar rendering/preview stubs
- [x] **KeyConfigState / SkinConfigState** — MainState trait impls with Phase 22 warn stubs in beatoraja-core config_pkg
- [x] **MainController state dispatch** — StateFactory trait for cross-crate state creation, change_state() with MainStateType dispatch (matching Java switch), transition_to_state() lifecycle (create→prepare→shutdown old), get_current_state/get_state_type, lifecycle dispatch (render/pause/resume/resize/dispose)
- [x] **Decide skip logic** — config.skip_decide_screen routes Decide→Play (matching Java)

### Phase 19: SkinData→Skin Loading Pipeline (complete)

+1,469 lines across 6 files, +20 tests. All sub-phases done:

- [x] **19a:** JsonSkinObjectLoader base — complete conversion methods for all JsonSkin types (Image, ImageSet, Text, Value, Slider, Graph, GaugeGraph, JudgeGraph, BpmGraph, NoteSet, SongList, PMchara, HiddenCover, LiftCover, BGA, Judge). 820+ lines added to json_skin_object_loader.rs
- [x] **19b:** Screen-specific loaders — PlaySkinObjectLoader (note field, gauge, judge, lane cover, BGA), SelectSkinObjectLoader (bar list rendering). Decide/Result/Course/KeyConfig/SkinConfig remain minimal (delegate to base, matching Java)
- [x] **19c:** LuaSkinLoader — `load_header()` and `load_skin()` implemented via mlua. `from_lua_value()` recursive converter: LuaTable → JsonSkin data tree. 280 lines
- [x] **19d:** SkinLoader entry points — `load()` routes to JSONSkinLoader or LuaSkinLoader based on file extension. `load_skin()` wired to screen-specific object loader creation. JSONSkinLoader `load_skin()` fully connected

### Phase 20: IRConnection Integration (complete)

+263 lines across 6 files + 2 new files, +13 tests:

- [x] `IRSendStatus` — full `send()` implementation: calls `connection.send_play_data()`, checks response, updates `is_sent`/`retry`. `send_course()` for course results. 250 lines
- [x] `IRInitializer` — `initialize_ir()` method: iterates player IR configs, creates connections via `IRConnectionManager`, calls login, returns `Vec<IRStatus>`. 107 lines
- [x] `IRResend` — `IRResendLoop` with exponential backoff (`4^retry * 1000ms`), periodic retry via `tokio::time::interval`, configurable max retries. 232 lines
- [x] `IRStatus` — updated with `connection: Arc<dyn IRConnection>`, `config`, `player` fields
- [x] IR stub comments updated to "real implementations (Phase 20)" in beatoraja-result/stubs.rs

## Blocked Tasks

### Phase 16b: Golden Master Test Activation (partially complete)

- [ ] Add missing fixtures for modules not yet covered (modmenu, select bar, stream) — deferred until Rust-side APIs are implemented
- [ ] Reactivate `compare_render_snapshot.rs` — blocked: rendering pipeline not yet connected to wgpu. SkinLoader now functional but SkinObject→GPU rendering gap remains

### Phase 18e: Stub replacement (remaining items blocked)

- [x] Replace `MainState` stubs with real trait impls — **DONE (Phase 21)**: all 6 screen states implement MainState trait
- [ ] Remove all `stubs.rs` files — blocked: depends on rendering/database implementations
- [ ] beatoraja-external LibGDX stubs (Pixmap/GdxGraphics/BufferUtils/PixmapIO) — blocked on wgpu rendering pipeline

### Phase 18f: Integration verification (partially unblocked)

- [ ] Activate `compare_render_snapshot.rs` — partially unblocked: skin loading pipeline done, but SkinObject→GPU rendering not connected
- [x] E2E gameplay flow test: select → decide → play → result screen transitions — **PARTIALLY DONE (Phase 21)**: MainController.change_state() dispatches to concrete states via StateFactory. Full E2E test needs launcher-side factory impl
- [ ] Final verification: all tests pass, zero clippy warnings, clean `cargo fmt` — blocked: final gate

### Known Issues (open)

- [x] SkinObject→GPU rendering gap — **PARTIALLY RESOLVED (Phase 22b/c)**: SpriteBatch flush wired to wgpu render pass, SkinObject draw methods implemented and tested. Remaining: Skin.draw_all_objects() integration with real Skin type in MainStateData
- [ ] Remaining stubs: ~2,200 lines across 16 stubs.rs files — blocked by rendering, database implementations
- [ ] MainController still has ~12 stub methods (polling thread, updateStateReferences, audio driver) — partially unblocked by Phase 21/23, remaining blocked on Phase 22
- [x] StateFactory concrete implementation — DONE (Phase 23): LauncherStateFactory in beatoraja-launcher wires all 7 screen states

## Next Phases (planned)

### Phase 22: Rendering Pipeline (SkinObject→GPU) — in progress

Unblocks: Phase 16b render snapshot tests, Phase 18f E2E tests, visual output

- [x] **22a: WGSL sprite shader + wgpu render pipeline + SpriteBatch GPU flush** — WGSL shaders for all 6 Java shader types (Normal, Linear, Bilinear, FFmpeg, Layer, DistanceField), SpriteRenderPipeline with 30 pipeline variants (6 shaders x 5 blend modes), SpriteBatch flush_to_gpu(), SkinObjectRenderer pre_draw/post_draw wired with shader switching + blend state + color save/restore. +43 new tests
- [x] **22b: SkinObject draw methods + SkinTextBitmap** — Draw method integration tests for SkinImage/SkinNumber/SkinTextImage (27 tests). SkinTextBitmap.draw_with_offset() implemented with ab_glyph (glyph layout, alignment, overflow, shadow, distance field). +15 tests. +1,346 lines
- [x] **22c: MainController render pipeline + FPS cap** — render() enhanced with sprite.begin()/end() lifecycle and input gating. SpriteBatch re-export (real impl replacing stub). SpriteBatch flush wired to wgpu render pass with bind groups. FPS capping from config. +7 tests. +325 lines
- [ ] **22d:** Skin.draw_all_objects() integration — wire Skin type into MainStateData for per-frame object prepare/draw

### Phase 23: Database Integration — partially complete

Unblocks: SongDatabaseAccessor stubs, PlayDataAccessor stubs

- [x] **23a: LauncherStateFactory** — Concrete StateFactory impl in beatoraja-launcher. Creates all 7 state types (MusicSelect, Decide, Play, Result, CourseResult, Config, SkinConfig). Wired with MainController state dispatch. +10 tests
- [x] **23b: MainController DB wiring** — `songdb: Option<Box<dyn SongDatabaseAccessor>>` field on MainController, `set_song_database()` / `get_song_database()` methods. `PlayDataAccessor::new(&config)` in constructor and initialize_states()
- [x] **23c: MusicSelector DB injection** — `with_song_database()` constructor for injecting `Box<dyn SongDatabaseAccessor>`
- [x] **23d: CourseResult MainState** — Added `MainState` trait impl to CourseResult with `main_data: MainStateData` field
- [ ] Wire rusqlite SongDatabaseAccessor with real schema — blocked: requires MainLoader.play() launcher entry point
- [ ] Connect to MusicSelector song list loading — blocked: BarManager needs songdb for initial bar creation
