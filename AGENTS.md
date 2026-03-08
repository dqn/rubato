# lr2oraja Rust Porting

beatoraja fork (Java 313 files / 72k+ lines) → Rust. 25 crates, 167k lines.

## Rules

- Golden Master: pre-generated JSON fixtures in `golden-master/fixtures/`. Tolerance: ±2μs.
- Preserve ALL branch/loop/fallthrough structure. Copy constants/magic numbers AS-IS.
- Explicit type conversions — every implicit Java cast → explicit Rust cast.
- After completing a phase/task, update TODO.md and AGENTS.md.
- Worktree isolation: **always merge worktree branches before sending shutdown requests**.
- Worktree cleanup: **always run `rm -rf target/` inside a worktree before removing it** to avoid multi-GB disk waste. Each worktree's `target/` can be 4+ GB.

## Type Mapping

| Java | Rust |
|------|------|
| `null` / `try-catch` | `Option<T>` / `Result<T>` + `anyhow` |
| `ArrayList` / `HashMap` / `TreeMap` | `Vec` / `HashMap` / `BTreeMap` (`TreeMap<Double>` → `BTreeMap<u64>` via `to_bits()`) |
| `synchronized` / `static` | `Mutex`/`RwLock` / `OnceLock` |
| Abstract class + `instanceof` | Enum + `Data` struct + `match` |
| Interface / Abstract factory | `Box<dyn Trait>` / Enum + `modify()` |
| POJO config | `pub` fields + `#[derive(Serialize, Deserialize)]` + `#[serde(default)]` |
| `PreparedStatement` + `ResultSet` | rusqlite `prepare` + `query_map` + `params![]` |
| `ByteBuffer.slice()` | `Arc<Vec<T>>` + offset/length |
| JavaFX `TableView<T>` | `Vec<T>` + `Vec<usize>` (selected) |

## Tech Stack

| Java | Rust |
|------|------|
| LibGDX / PortAudio | wgpu / Kira 0.12 |
| LuaJ / SQLite (JDBC) | mlua / rusqlite |
| JavaFX / ImGui | egui (egui-wgpu 0.31) |
| Discord (JNA) / OBS (WebSocket) | discord-rich-presence / tokio-tungstenite |

## Structure

```
rubato/              # Cargo workspace (15 crates) at repo root
  crates/
    bms-model        # BMS/BME/BML parser + model
    bms-table        # Difficulty table parser
    rubato-types     # Shared types (circular dep breaker)
    rubato-audio     # Audio (Kira 0.12)
    rubato-input     # Keyboard/controller input (+ controller)
    rubato-render    # Rendering (wgpu)
    rubato-skin      # Skin loading/layout
    rubato-song      # Song DB (rusqlite, + md-processor)
    rubato-core      # State machine, main loop (+ pattern)
    rubato-play      # Play state (gameplay)
    rubato-state     # Select/Decide/Result/Modmenu/Stream states
    rubato-ir        # Internet ranking
    rubato-external  # Twitter, clipboard, Discord RPC, OBS WebSocket
    rubato-launcher  # Launcher UI (egui)
    rubato-bin       # Entry point
  golden-master/   # Golden Master test infra
  test-bms/        # Test BMS files
```

## Key Invariants

- Timing: i64 microseconds. JavaRandom LCG in `rubato-core::pattern` (**never** `StdRng`/`rand`). LR2 MT19937. LR2 judge: pure integer arithmetic. LongNote: index-based.

## Testing

- **Test runner:** `just test` (excludes slow render snapshot tests) or `just test-all` (full).
- **Golden Master:** `just golden-master-test`. Fixtures: `golden-master/fixtures/*.json` (pre-generated).
- **TDD:** Red-Green-Refactor.

## Status

**4146 tests.** Phases 1–62 complete + post-62 stub audit + hardening pass + Phase 9 launcher egui + E2E lifecycle tests + bug-fix & test hardening pass (3 rounds) + round 4 bug fixes, tests, safety audit, fuzz targets + criterion benchmarks + performance optimization (BMS decoder, pattern modifier, SpriteBatch) + functional gap fixes (target score, BGI maxgen, LR2 play skin loader) + robustness hardening (bounds checks, div-by-zero guards, overflow prevention, PCM/skin_gauge test expansion, allow(unused) removal, panic→Result, fuzz targets) + quality hardening round 3 (Regex OnceLock, clippy allow removal across 13 crates, Color/KeyInputLog/RandomTrainer test expansion) + quality hardening round 4 (pomyu_chara_loader bounds safety, O(n²)→HashSet, OBS Mutex lock_or_recover, skin_property_mapper docs, input→judge integration tests) + quality hardening round 5 (draw loop clone removal, Vec::with_capacity hints, unwrap→expect in parsers, 34 crate-level allow directives removed across 14 crates). Zero clippy warnings. Zero regressions.
**Migration audit**: 4,279 Java methods: 4,049 resolved, 230 architectural redesigns (getter→pub field, Thread→spawn, Gson→serde, etc.). 0 remaining functional gaps. ast-compare retired (final verification: 0 missing methods).
**"Not implemented" stubs**: 0 remaining. All 151 stubs resolved (Phase 58–62).
**Debug stubs**: 0 remaining. All 32 resolved: 12 implemented, 20 → compile-time comments.

### Resolved (Phase 58–62 + Post-62)

- **Phase 58**: 46 test/null/out-of-scope stubs reclassified, 18 blocked stubs documented
- **Phase 59**: Sound system wiring (SoundType, MusicSelector events, sound overrides)
- **Phase 60**: PlayerResource reverse lookup, trait expansion, MainController Box::leak eliminated, ChartReplication
- **Phase 61**: OBS triggerStateChange implemented, LR2 CSV INCLUDE, 35 blocked stubs downgraded
- **Phase 62**: 10 launcher egui stubs downgraded with blocker descriptions
- **Post-62a**: 9 debug stubs implemented — RankingDataCache (real HashMap cache), open_ir (browser), Target cycling, FavoriteSong/Chart, OpenDocument/WithExplorer/DownloadSite
- **Post-62b**: OpenIr in MusicSelector (via MainControllerAccess IR URL methods), CIM image fallback, all 31 debug stubs → compile-time comments (0 runtime stubs)
- **Post-62c**: SongSelectionAccess trait (modmenu↔select bridge), SkinRenderContext trait (SkinDrawable expansion), DistributionData (SkinDistributionGraph bridge), external property factory adapters
- **Post-62d**: egui launcher UI (Audio/Discord/OBS tabs, What's New/Chart Details popups, search popup InputEvent), wgpu rendering pipeline (CourseResult skin+fadeout, SkinDistributionGraph draw methods, MessageRenderer draw wiring), LR2 SkinObject assembly (assemble_objects() trait on all 6 loaders — play: judge/note/bga/line, select: bar)
- **Post-62e**: Select crate stubs replaced with real rubato-skin types (SkinImage, SkinNumber, SkinText trait, SkinObjectRenderer, SkinRegion→Rectangle alias). GlyphAtlas + BitmapFont::draw() implemented with row-packing atlas and SpriteBatch glyph rendering. MessageRenderer wired with SpriteBatch + alpha pulsing animation.

### Permanent Stubs (intentionally unimplemented)

- **Twitter4j** (`rubato-external`): ~446 lines, `bail!()` — API deprecated
- **ShortDirectPCM** (`rubato-audio`): Java-specific DirectBuffer — unnecessary in Rust
- **JavaFX find_parent_by_class_simple_name** (`rubato-launcher`): No egui equivalent
- **randomtrainer.dat** (`rubato-modmenu`): Binary resource from Java, uses empty HashMap fallback

## Lessons Learned

- **Encoding:** `encoding_rs::SHIFT_JIS` for MS932. **Serde:** `BPM`→`Bpm`, `URL`→`Url`, `#[serde(alias)]`.
- **Borrow checker:** `&mut` conflicts → scoped block. Self-ref → `Option::take()`. Parent ref → callback trait.
- **Stubs:** `stubs.rs` per crate → replace via `pub use`. Always `cargo check` after removal.
- **Circular deps:** `rubato-types` for shared types. Core cannot import: song, skin, play, select, result, ir, modmenu.
- **Lua→JSON coercion:** 3-layer: numbers→strings, float→int truncation, empty `{}`→remove.
- **Bar Clone:** `Box<dyn Trait>` blocks Clone → use `Arc<dyn Trait>` for shared trait objects.
- **Property delegate pattern:** `integer_value(id)` / `float_value(id)` / `boolean_value(id)` on MainState — skin property factories delegate via ID lookup.
- **Java float→int→byte truncation:** Use `as i32 as i8` in Rust (via i32 to get truncation). Direct `as i8` saturates since Rust 1.45.
- **Controller wiring across crates:** When states cannot own `&mut MainController`, use a queued `MainControllerAccess` proxy plus a MainController-side drain step instead of config-only no-op adapters. Shared `Arc<Mutex<State>>` wrappers must explicitly sync `MainStateData` back and forth or skins/timers will desynchronize.
- **Skin event wiring:** `SkinDrawable` mouse/custom-event paths need a state-aware `SkinRenderContext`, not a timer-only adapter. Delegate mutating methods (`execute_event`, `change_state`, config writes, float writes) through the adapter, and let interactive states like `MusicSelector` build their own context for mouse/render passes.
- **Mouse bridge parity:** Having a live render context is not enough. States with clickable skins must also override `handle_skin_mouse_pressed()` / `handle_skin_mouse_dragged()`, temporarily take ownership of `skin` and `TimerManager`, and build the same kind of state-aware context for the mouse pass or replay buttons / play-screen option clicks will silently no-op.
- **Analog input handoff:** For `BMSPlayerInputProcessor` proxies, snapshot `is_analog`, `get_analog_diff()`, and `get_time_since_last_analog_reset()` during `sync_input_from()`, then flush consumed keys back with `reset_analog_input()` in `sync_input_back_to()`.
- **Resource path resolution:** Skin/config asset paths may be stored relative to the workspace root or to `config.skinpath`. Resolve against both the configured skin root and ancestor directories of the current working directory so tests and sub-crate launches do not blank-screen on missing skins.
- **Play input handoff:** `BMSPlayer` does not read `MainController` input implicitly. Rust-side state hooks must explicitly copy START/SELECT/key/control/scroll/device state from `BMSPlayerInputProcessor`, and write back consumed flags like START/SELECT/scroll after processing.
- **State-owned audio side effects:** States that need the real `AudioDriver` after render/shutdown should flush them through `MainState::sync_audio()` from `MainController`, not through queued `MainControllerAccess` commands. Otherwise preview music and shutdown-time stop/dispose paths can silently no-op or double-fire against stale queued state.
- **Ranking cache sharing:** IR ranking cache handles must stay shared across `MainController`, queued launcher access proxies, and result wrappers. Fresh per-wrapper caches break select→play/result reuse and force redundant IR fetches or empty ranking views after transitions.
- **GUI subprocess smoke tests:** Launcher/play subprocess tests must use an explicit timeout instead of `Command::output()` alone. On machines with a working display the "success" path can keep the UI event loop alive indefinitely, which turns ignored smoke tests and `Justfile` recipes into hangs instead of pass/fail signals.
- **Skin property ref parity:** LR2 numeric refs and image-index refs can reuse the same ID for different meanings. Keep `integer_value(id)` and `image_index_value(id)` separate, or shipped skins will render the wrong mode/sort/random/target frame even when the underlying numeric state is correct.
- **Target list fallback:** Built-in target cycling and target image refs must not assume `TargetProperty`'s global list is initialized. When the global list is empty, fall back to `player_config.targetlist` so play/result screens and isolated tests do not silently no-op.
- **Selected play-config lookup:** Java `getSelectedBarPlayConfig()` semantics depend on the selected song/course mode, not only the current selector mode. Rust ports must resolve play config from the selected bar's actual mode (and only share course config when every song maps to the same bucket), or lane-cover/judge-option skin refs will point at the wrong config.

## Landing the Plane (Session Completion)

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Sync beads** - `bd sync`
5. **Clean up** - Clear stashes, prune remote branches
6. **Hand off** - Provide context for next session
