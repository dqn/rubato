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

All phases complete. **1511 tests pass. Zero runtime `todo!()`/`unimplemented!()`.** PlayerResource wrapper migration complete for all 6 crates.

- **Phases 1–17:** Core translation (17 crates, 300+ modules), real implementations (wgpu, Kira, mlua, ffmpeg-next, midir, cpal, egui UI), circular dep resolution, stub cleanup, platform replacements, 868 tests (715 unit + 121 golden master + 32 integration)
- **Phase 18a–d:** Core judge loop, rendering state providers, audio decode API, BGA/skin test APIs
- **Phase 18e (1–12):** Stub replacement — 12 sub-phases of cross-crate dedup, lifecycle cleanup, PlayerResource wrapper, skin/input/IR/table type replacements, dependency cleanup. 4 rounds of full audit — all actionable replacements exhausted
- **Phase 18f:** E2E test activation (138 tests across 9 files)
- **Phase 18g:** BRD replay codec
- **Phase 19:** SkinData→Skin Loading Pipeline — JsonSkinObjectLoader base conversion (all skin object types), screen-specific loaders (Play/Select + 5 minimal), LuaSkinLoader (mlua-based Lua→JsonSkin), SkinLoader entry points. +1,469 lines, +20 tests
- **Phase 20:** IRConnection Integration — IRSendStatus.send() with score submission, IRInitializer for connection setup/login, IRResendLoop with exponential backoff (tokio), IRStatus with real connection type. +263 lines + 2 new files, +13 tests
- **Phase 21:** Per-Screen MainState + State Dispatch — All 6 screen states (MusicSelector, MusicDecide, BMSPlayer, MusicResult, KeyConfiguration, SkinConfiguration) implement MainState trait. MainController state dispatch via StateFactory trait (cross-crate), change_state() with Java-matching switch logic, transition lifecycle (create/prepare/shutdown), lifecycle dispatch (render/pause/resume/resize/dispose), decide-skip logic. +23 tests
- **Phase 22a:** WGSL Sprite Shader + Render Pipeline + SpriteBatch GPU Flush — WGSL shaders for all 6 Java shader types (Normal, Linear, Bilinear, FFmpeg, Layer, DistanceField), SpriteRenderPipeline with 30 pipeline variants (6 shaders x 5 blend modes), SpriteBatch flush_to_gpu() with vertex buffer upload, SkinObjectRenderer pre_draw/post_draw wired (shader switching, blend state, color save/restore). +43 tests
- **Phase 22b:** SkinObject Draw Methods + SkinTextBitmap — Integration tests for SkinImage/SkinNumber/SkinTextImage draw chains (27 tests). SkinTextBitmap.draw_with_offset() implemented with ab_glyph font rendering (glyph layout with kerning, alignment, overflow modes, shadow, distance field). BitmapFont.layout_glyphs() + PositionedGlyph. +42 tests, +1,346 lines
- **Phase 22c:** MainController Render Pipeline + FPS Cap — MainController.render() enhanced (sprite begin/end lifecycle, input gating by time delta). SpriteBatch re-export replacing stub. SpriteBatch→wgpu render pass flush with SpriteRenderPipeline, bind groups, projection matrix. FPS capping from Config.maxFramePerSecond. +7 tests, +325 lines
- **Phase 22d:** Skin.draw_all_objects() Integration — SkinDrawable trait in beatoraja-core (Send-bounded, 10 methods), TimerOnlyMainState adapter bridging core↔skin MainState, impl SkinDrawable for Skin, MainController.render() wired with take/put-back borrow pattern, SkinStub removed from MainStateData. +11 tests, +~150 lines
- **Phase 23a–d:** LauncherStateFactory + DB wiring — LauncherStateFactory concrete impl in beatoraja-launcher (all 7 state types), MainController `songdb` field + `set_song_database()`/`get_song_database()`, PlayDataAccessor init in constructor, MusicSelector `with_song_database()` injection, CourseResult MainState trait impl. +10 tests
- **Phase 24c:** Audio driver wiring — beatoraja-select AudioDriver stub deleted, PreviewMusicProcessor wired to `&dyn AudioDriver` trait (beatoraja-audio), MainController `audio: Option<Box<dyn AudioDriver>>` field + get/set methods. +11 tests
- **Phase 24d:** RenderSnapshot test activation — Fixed imports (`bms_config`→`beatoraja_core`, `bms_render`→`golden_master`, `bms_skin`→`beatoraja_skin`), added `Gauge` DrawDetail variant + comparison, moved from `tests/pending/` to `tests/`, 22 tests compiled (#[ignore] — SkinData→Skin pipeline needed)

## Remaining Stubs (~2,613 lines across 16 files)

- **beatoraja-external (574 lines):** Pixmap/GdxGraphics/BufferUtils/PixmapIO LibGDX stubs — Phase 22 で wgpu パイプラインが完成し代替実装が可能に。Phase 24 で段階的に解消予定
- **beatoraja-result (388 lines):** SkinObjectData, AbstractResult/ScreenType stubs — 大半は re-export、rendering 関連はPhase 22 で解消可能
- **beatoraja-select (~348 lines):** EventType enum、SkinObject/SkinNumber/SkinText/SkinImage rendering stubs、SongManagerMenu wrapper、DownloadTask stubs (AudioDriver stub deleted in Phase 24c)
- **beatoraja-launcher (321 lines):** MainController partial stubs — Phase 24f で解消
- **beatoraja-skin (294 lines):** LibGDX graphics stubs (Color, Texture, TextureRegion, Pixmap) — wgpu 型への置換が可能だが全 crate に影響
- **beatoraja-types (211 lines):** 基本型スタブ — 多くは re-export 用
- **beatoraja-modmenu (191 lines):** SongManagerMenu, MusicSelector/Bar rendering — modmenu 完全実装まで保持
- **beatoraja-input (132 lines):** Config/PlayerConfig/PlayModeConfig stubs — 入力システム統合 (Phase 24b) で解消
- **beatoraja-decide (108 lines):** SkinStub — Phase 22d で MainStateData から除去済み、残は decide 固有
- **Clean crates (re-exports only):** beatoraja-core (1), beatoraja-audio (1), beatoraja-play (9), beatoraja-obs (9), beatoraja-ir (10), md-processor (12), beatoraja-stream (4)
- **MainController:** ~11 stub methods — polling thread (Phase 24b), audio driver init (Phase 24f, driver field wired in Phase 24c), updateStateReferences (Phase 24f)
- **StateFactory:** DONE — LauncherStateFactory in beatoraja-launcher wires all 7 screen state types
- **Platform:** Windows named pipe (not yet implemented)
- **Intentional:** Twitter4j → `bail!()` (永久)

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
