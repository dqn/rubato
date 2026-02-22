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

All phases complete. **1274 tests pass. Zero runtime `todo!()`/`unimplemented!()`.** PlayerResource wrapper migration complete for all 6 crates.

- **Phases 1–17:** Core translation (17 crates, 300+ modules), real implementations (wgpu, Kira, mlua, ffmpeg-next, midir, cpal, egui UI), circular dep resolution, stub cleanup, platform replacements, 868 tests (715 unit + 121 golden master + 32 integration)
- **Phase 18a–d:** Core judge loop, rendering state providers, audio decode API, BGA/skin test APIs
- **Phase 18e (1–12):** Stub replacement — 12 sub-phases of cross-crate dedup, lifecycle cleanup, PlayerResource wrapper, skin/input/IR/table type replacements, dependency cleanup. 4 rounds of full audit — all actionable replacements exhausted
- **Phase 18f:** E2E test activation (138 tests across 9 files)
- **Phase 18g:** BRD replay codec
- **Phase 19:** SkinData→Skin Loading Pipeline — JsonSkinObjectLoader base conversion (all skin object types), screen-specific loaders (Play/Select + 5 minimal), LuaSkinLoader (mlua-based Lua→JsonSkin), SkinLoader entry points. +1,469 lines, +20 tests
- **Phase 20:** IRConnection Integration — IRSendStatus.send() with score submission, IRInitializer for connection setup/login, IRResendLoop with exponential backoff (tokio), IRStatus with real connection type. +263 lines + 2 new files, +13 tests

## Remaining Stubs (~2,540 lines across 16 files, all blocked)

- **MainController:** ~20 stub methods (state transitions, state management, database access — blocker: Phase 21), md-processor (intentional adapter, deferred), modmenu (3 methods, until real MainController)
- **Rendering:** SkinText/SkinNumber/SkinImage/SkinObject/SkinObjectRenderer (select), Skin/SkinObject/Rectangle (modmenu), SkinStub (decide), SkinObjectData (result), LibGDX stubs (external) — all blocked on rendering pipeline (Phase 22)
- **Per-screen:** MainState trait impls, EventType/AudioDriver (select), AbstractResult/ScreenType (external), MusicSelector/Bar/SongBar (modmenu) — blocked on Phase 21
- **Other:** Twitter4j (intentional bail), Property stubs (MainState type mismatch), ScoreDatabaseAccessor (external — Phase 23), DownloadTask (select)
- **Clean crates:** beatoraja-obs/stream/ir/md-processor/pattern (re-exports only, zero real stubs)
- **Platform:** Windows named pipe (not yet implemented)

## Lessons Learned

### Core Patterns
- **MS932:** `encoding_rs::SHIFT_JIS.decode(raw_bytes)`. LR2IR: Shift_JIS HTTP via `encoding_rs`, XML via `quick-xml`.
- **Borrow checker:** Parent `this` ref → callback trait (`&mut dyn Trait`). Constructor with sibling → pass primitives. `&mut` borrow conflicts → scoped block pattern (collect results into locals first).
- **Stubs:** Forward stubs in `stubs.rs` per crate. Replace via `pub use real_crate::module::Type;`. Add Java-style getters to real types rather than modifying callers. Always `cargo check` after removal.
- **Circular deps:** Core cannot import: song, skin, play, select, result, ir, modmenu. Solution: `beatoraja-types` crate; core re-exports via `pub use`. Extract minimum shared state to break cycles.
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
