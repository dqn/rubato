# lr2oraja Rust Porting — Mechanical Line-by-Line Translation

lr2oraja (beatoraja fork, Java 313 files / 72k+ lines) → Rust. All features incl. peripherals in scope.
**FRESH START. All previous Rust code discarded.**

## Rules

- **NEVER** explore/investigate/plan. Workflow: `Read Java → Write Rust → Test → Next`.
- **NEVER** read previous implementation/plans/old commits. ONLY source: `./lr2oraja-java`.
- **ZERO improvements** — copy Java verbatim. Refactor ONLY after ALL tests pass.
- **Translate one method → test immediately** — green before moving on.
- **Golden Master** — export Java values as JSON, compare with Rust. Tolerance: ±2μs.
- **Preserve ALL branch/loop/fallthrough structure.** Copy constants/magic numbers AS-IS.
- **Explicit type conversions** — every implicit Java cast → explicit Rust cast.
- When all actionable tasks are blocked, investigate and plan the next phases, then add them to TODO.md.
- After completing a phase/task, update TODO.md (progress, new issues, deferred items) and AGENTS.md (Implementation Status, Remaining Stubs).
- When new issues are discovered or deferred, add them to TODO.md immediately.
- When tasks become unblocked, update their blocker status in TODO.md.
- When using worktree isolation for team agents, **always merge worktree branches before sending shutdown requests**.

## Type Mapping

| Java | Rust |
|------|------|
| `null` check / `try-catch` | `Option<T>` / `Result<T>` + `anyhow` |
| `ArrayList<T>` / `HashMap<K,V>` | `Vec<T>` / `HashMap<K,V>` |
| `TreeMap<K,V>` / `TreeMap<Double,V>` | `BTreeMap<K,V>` / `BTreeMap<u64,V>` via `to_bits()` |
| `TreeMap.lowerEntry(y)` | `BTreeMap::range(..y).next_back()` |
| `synchronized` / `static` field | `Mutex`/`RwLock` / `lazy_static!`/`OnceLock` |
| Abstract class + `instanceof` | Enum + shared `Data` struct + `match` |
| Interface with lambdas | Enum + `modify()`, or `Box<dyn Trait>` |
| Abstract class + factory | Trait + `Box<dyn Trait>` factory |
| POJO config | `pub` fields + `#[derive(Serialize, Deserialize)]` + `#[serde(default)]` |
| `@JsonIgnoreProperties` / LibGDX Json | `serde_json::from_str` + `#[serde(default)]` |
| `PreparedStatement` + `ResultSet` | rusqlite `prepare` + `query_map` + `params![]` |
| `ByteBuffer.slice()` | `Arc<Vec<T>>` + offset/length |
| `TextureRegion[]` (nullable) | `Vec<Option<TextureRegion>>` |
| `java_websocket.WebSocketClient` | tokio + `futures_util::SplitSink/SplitStream` |
| JavaFX views / `TableView<T>` | Plain structs; `Vec<T>` + `Vec<usize>` (selected) |

## Tech Stack

| Java | Rust |
|------|------|
| LibGDX (LWJGL3) / PortAudio | wgpu / Kira 0.12 |
| LuaJ / SQLite (JDBC) | mlua / rusqlite |
| JavaFX / ImGui | egui (egui-wgpu 0.31) |
| JNA IPC (Discord) / WebSocket (OBS) | discord-rich-presence / tokio-tungstenite |
| long (μs) | i64 (μs) |

## Directory & Structure

```
brs/
  lr2oraja-java/    # Java source (read-only except debug output)
  lr2oraja-rust/    # Cargo workspace
    crates/         # Rust crates
    golden-master/  # Test infra (Java exporter + fixtures)
    test-bms/       # Test BMS files
```

## Key Invariants

- Timing: i64 microseconds, no floating-point drift.
- `java.util.Random(seed)` LCG: multiplier=`0x5DEECE66D`, addend=`0xB`, mask=`(1L<<48)-1`. Reproduce exactly. Implemented as `java_random::JavaRandom` in `beatoraja-pattern`. **Never use `StdRng`/`rand`.**
- LR2 Mersenne Twister: custom MT19937, LR2-specific seeding, `u32` wrapping arithmetic.
- LR2 judge scaling: pure integer arithmetic. LongNote refs: index-based.

## Testing

- **Golden Master:** Java state → JSON → Rust comparison. Java BMSDecoder hardcodes MS932. `#RANDOM` deterministic via `random_seeds.json`. Regenerate: `just golden-master-gen`. Fixtures: `filename.ext.json`.
- **TDD:** Red-Green-Refactor for every method. Java mods allowed for debug/JSON export.
- **ast-compare:** Static structural verification tool (`lr2oraja-rust/crates/ast-compare`). tree-sitter で Java/Rust AST をパースし、シグネチャマッピング・制御フロー構造比較・定数検査を行う。`just ast-map` / `just ast-compare` / `just ast-constants` / `just ast-full` で実行。getter/setter→pub field マッチ、コンストラクタオーバーロード、グローバル型検索（`pub use` 再エクスポート対応）で偽陽性を削減済み。

## Implementation Status

Phases 1–24e complete. **1651 tests pass, 22 ignored (RenderSnapshot — SkinData→Skin pipeline pending). Zero runtime `todo!()`/`unimplemented!()`.** PlayerResource wrapper migration complete for all 6 crates. Phase 24f (MainController remaining stubs) next.

- **Phases 1–17:** Core translation (17 crates, 300+ modules), real implementations (wgpu, Kira, mlua, ffmpeg-next, midir, cpal, egui UI), circular dep resolution, stub cleanup, platform replacements, 868 tests
- **Phase 18a–g:** Core judge loop, rendering state providers, audio decode API, BGA/skin test APIs, stub replacement (12 sub-phases, 4 audit rounds), E2E test activation (138 tests), BRD replay codec
- **Phase 19:** SkinData→Skin Loading Pipeline — JsonSkinObjectLoader, screen-specific loaders, LuaSkinLoader, SkinLoader entry points. +1,469 lines, +20 tests
- **Phase 20:** IRConnection Integration — IRSendStatus, IRInitializer, IRResendLoop. +263 lines, +13 tests
- **Phase 21:** Per-Screen MainState + State Dispatch — 6 screen states, StateFactory trait, change_state(), lifecycle dispatch. +23 tests
- **Phase 22a–d:** Rendering Pipeline — WGSL shaders (6 types, 30 variants), SkinObject draw + SkinTextBitmap (ab_glyph), MainController render + FPS cap, Skin.draw_all_objects() (SkinDrawable trait). +103 tests, +1,821 lines
- **Phase 23a–d:** LauncherStateFactory + DB wiring — 7 state types, songdb field, CourseResult. +10 tests
- **Phase 24a:** SQLiteSongDatabaseAccessor + MainLoader — 6/6 trait methods, updateSongDatas(), LauncherUi eframe. +710 lines, +23 tests
- **Phase 24c:** Audio driver wiring — AudioDriver stub deleted, MainController audio field. +11 tests
- **Phase 24b:** Input system integration — WinitKeyCode→Java keycode mapping, SharedKeyState, GdxInput/GdxGraphics real impl, MainController input wiring. +46 tests
- **Phase 24d:** RenderSnapshot test activation — 22 tests compiled (#[ignore])
- **Phase 24e:** BarManager + music selection — init/update_bar/close, BarContentsLoaderThread, UpdateBarContext/LoaderContext/CourseTableAccessor. +40 tests

## Remaining Stubs (3,004 lines across 16 files)

Breakdown: ~1,520 lines true stubs, ~1,100 lines already-implemented code living in stubs.rs, ~140 lines re-exports, ~244 lines tests.

### Large stubs.rs files (actual stubs needing resolution)

- **beatoraja-external (936 lines):** Pixmap/GdxGraphics/BufferUtils/PixmapIO are real implementations (~425 lines + 244 test lines) using `image` crate + atomic globals — should be moved to proper modules. ClipboardHelper (~25 lines) uses `arboard`. Remaining true stubs: ScoreDatabaseAccessor (~15 lines), MainState struct + ScreenType + AbstractResult (~60 lines), IntegerProperty/BooleanProperty/StringProperty traits + factories (~70 lines), Twitter4j (~130 lines, intentional `bail!()`), MainStateListener trait (~5 lines)
- **beatoraja-result (388 lines):** 25 re-exports. MainController stub (10 methods, ~70 lines), PlayerResource wrapper (35 delegation methods, ~170 lines), RankingDataCache (~20 lines), AudioProcessorStub (~8 lines), SkinObjectData (~5 lines)
- **beatoraja-launcher (355 lines):** Much is real code: rfd file dialogs (~30 lines), `open` crate URL/folder (~10 lines), arboard clipboard (~15 lines), cpal audio device enum (~20 lines), CoreGraphics FFI monitor enum (~70 lines). True stubs: MainLoader display stubs (~10 lines), VersionChecker (~15 lines), SongDatabaseUpdateListener (~20 lines), TwitterAuth (~25 lines, intentional)
- **beatoraja-select (343 lines):** 41 re-exports. MainState trait (~3 lines), EventType enum (~25 lines), SkinText/SkinNumber/SkinImage/SkinObject rendering stubs (~90 lines), SkinObjectRenderer stub (~10 lines), SongManagerMenu wrapper (~10 lines), DownloadTask stubs (~40 lines)
- **beatoraja-skin (294 lines):** 3 re-exports + rendering_stubs.rs (15 lines, all re-exports). MainState/MainController/InputProcessor/Timer/Resolution/SkinOffset stubs (~105 lines), BMSPlayer/JudgeManager stubs (~40 lines), MusicResult/MusicResultResource/TimingDistribution stubs (~55 lines), PlayerResource stub (~35 lines), PlaySkinStub/SkinLoaderStub (~25 lines)
- **beatoraja-types (211 lines):** 3 re-exports. JudgeAlgorithm/BMSPlayerRule enums (~55 lines), BarSorter (~25 lines), scroll_speed/long_note/mine_note modifier stubs (~45 lines), IRConnectionManager (~15 lines), bms_player_input_device/KeyInputLog/PatternModifyLog (~40 lines)
- **beatoraja-modmenu (191 lines):** 15 re-exports. MainController (3 methods, ~15 lines), MainState trait (~5 lines), Skin/SkinObject/SkinObjectDestination/Rectangle stubs (~55 lines), MusicSelector/Bar/SongBar stubs (~40 lines)
- **beatoraja-input (132 lines):** 4 re-exports from beatoraja-types. Real code: GdxInput/GdxGraphics using SharedKeyState (~60 lines), Keys constants (~55 lines), Controller/SkinWidgetManager stubs (~15 lines) — mostly real, Phase 24b will complete
- **beatoraja-decide (108 lines):** 4 re-exports. MainControllerRef (3 methods, ~20 lines), AudioProcessorStub (~8 lines), SkinStub (~40 lines), load_skin/play_sound (~10 lines)

### Clean crates (re-exports or comments only)

beatoraja-core (1 line), beatoraja-audio (1 line comment), beatoraja-play (9 lines, 2 re-exports), beatoraja-ir (10 lines, 5 re-exports), beatoraja-obs (9 lines, 1 re-export + 2 constants), md-processor (12 lines, 2 re-exports + trait), beatoraja-stream (4 lines, 1 re-export)

### Other stubs (outside stubs.rs)

- **MainController:** 5 stub methods in main_controller.rs — application exit, SongUpdateThread (x2), updateTable, downloadIpfsMessageRenderer → Phase 24f
- **StateFactory:** DONE — LauncherStateFactory in beatoraja-launcher
- **Platform:** Windows named pipe (not yet implemented) → Phase 27a
- **Intentional:** Twitter4j → `bail!()` (~130 lines in external, ~25 lines in launcher — permanent)

## Lessons Learned

### Core Patterns
- **MS932:** `encoding_rs::SHIFT_JIS.decode(raw_bytes)`. LR2IR: Shift_JIS HTTP via `encoding_rs`, XML via `quick-xml`.
- **Borrow checker:** Parent `this` ref → callback trait (`&mut dyn Trait`). Constructor with sibling → pass primitives. `&mut` borrow conflicts → scoped block pattern (collect results into locals first). Owned-field access during self-reference → `Option::take()` + put-back pattern (e.g., skin inside MainStateData).
- **Stubs:** Forward stubs in `stubs.rs` per crate. Replace via `pub use real_crate::module::Type;`. Add Java-style getters to real types rather than modifying callers. Always `cargo check` after removal.
- **Circular deps:** Core cannot import: song, skin, play, select, result, ir, modmenu. Solution: `beatoraja-types` crate; core re-exports via `pub use`. Extract minimum shared state to break cycles. For state dispatch: `StateFactory` trait in core, concrete impl in launcher. For skin drawing: `SkinDrawable` trait in core, impl in skin crate, adapter bridges internal MainState stub.
- **State dispatch:** Java `instanceof` switch → Rust `MainState::state_type()` + `StateFactory` trait. Core holds `Box<dyn MainState>`, launcher provides factory. `change_state(MainStateType)` creates via factory, `transition_to_state()` handles lifecycle.
- **Parallel agents:** Independent crates → parallel agents. Create workspace `Cargo.toml` + all crate scaffolding BEFORE launching.

### API Incompatibility (Stub → Real)

| Mismatch | Fix |
|---|---|
| `String` vs `Option<String>` | `.unwrap_or_default()` or add getter |
| `i32` vs `Mode` | Update callers or `pub use Mode as Alias` |
| Struct vs Trait/Enum | `Box<dyn Trait>` or update to enum method calls |
| `&self` vs `&mut self` | Scoped block for borrow conflicts, `Box::leak` for `&mut T` returns |
| `set_field(v)` → pub field | Direct assignment |

### Key Technical Decisions
- **winit:** `create→resumed`, `render→RedrawRequested`, `ControlFlow::Poll`.
- **wgpu:** Direct (not Bevy). `pollster::block_on()` for async init.
- **Kira 0.12** (v3 doesn't exist).
- **mlua:** `load("return " + script).into_function()`. `package.loaded` for `require()`.
- **todo!()→warn!():** `OnceLock` for `&T`, `Box::leak` for `&mut T`. Default: `Mode::BEAT_7K`.
- **Serde camelCase:** `BPM`→`Bpm`, `URL`→`Url`, `SE`→`Se`, `RPC`→`Rpc`. `#[serde(alias)]` for variants.
- **HashMap ordering:** Deterministic tie-breaking (sort by BPM, use `>=`) for `mainbpm`.
- **Bounds checking:** `TimeLine::get_note`/`exist_note_at` need `.get()`.
- **CRC32:** Custom poly `0xEDB88320`, appends `\\\0`. **RobustFile:** double-write + `sync_all()`.
- **Platform:** Twitter4j → `bail!()`, AWT → `arboard`, PortAudio → `cpal`, monitors → CoreGraphics FFI (macOS) + winit.
- **egui:** `LauncherApp` (Wait) + `BeatorajaApp` (Poll). `RenderPass::forget_lifetime()` for wgpu 24.
- **PlayerResource:** Trait (32 methods) + wrapper struct for crate-local non-trait fields. `NullPlayerResource` for defaults.
- **build_judge_notes():** Sort by `(time_us, lane)`, remap `pair_index` via old→new index map.
- **BRD replay:** Gzip-compressed JSON. `write_brd()` → `shrink()` first. `read_brd()` → `validate()` after.
