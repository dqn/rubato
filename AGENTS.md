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
| LibGDX (LWJGL3) / PortAudio | Bevy / Kira |
| LuaJ / SQLite (JDBC) | mlua / rusqlite |
| JavaFX / ImGui | egui |
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
- `java.util.Random(seed)` LCG: multiplier=`0x5DEECE66D`, addend=`0xB`, mask=`(1L<<48)-1`. Reproduce exactly.
- LR2 Mersenne Twister: custom MT19937, LR2-specific seeding, `u32` wrapping arithmetic.
- LR2 judge scaling: pure integer arithmetic. LongNote refs: index-based.

## Testing

- **Golden Master:** Java state → JSON → Rust comparison. Java BMSDecoder hardcodes MS932. `#RANDOM` deterministic via `random_seeds.json`. Regenerate: `just golden-master-gen`. Fixtures: `filename.ext.json`.
- **TDD:** Red-Green-Refactor for every method. Java mods allowed for debug/JSON export.

## Implementation Status

| Phase | Crates | Modules |
|-------|--------|---------|
| 1 | `bms-model`, `bms-table` | 26 |
| 2 | `bmson`, `osu` | 27 |
| 3 | `beatoraja-common`, `discord-rpc`, `beatoraja-input`, `beatoraja-audio`, `md-processor` | 39 |
| 4 | `beatoraja-core` | 47 |
| 5 | `beatoraja-pattern`, `beatoraja-play` | 42 |
| 6 | `beatoraja-skin` | 50+ |
| 7 | `beatoraja-select`, `beatoraja-result`, `beatoraja-decide` | 39 |
| 8 | `beatoraja-ir`, `beatoraja-external`, `beatoraja-obs`, `beatoraja-modmenu`, `beatoraja-stream` | 41 |
| 9 | `beatoraja-launcher` | 21 |
| 10 | `beatoraja-song`, `beatoraja-controller`, `beatoraja-system` | 12 |
| 11 | Integration & wiring (stub replacement across 12 crates) | — |
| 12 | `beatoraja-bin` (CLI + winit event loop) | — |
| 14 | `beatoraja-types` (15 modules, circular dep resolution) | 15 |
| 15a | SongData/SongInformation/IpfsInformation → `beatoraja-types` | 3 |
| 15b | SkinType/GrooveGauge/GaugeProperty → `beatoraja-types` | 7 |
| 15c | Struct-vs-Trait Unification (SongDatabaseAccessor, IRConnection, BMSPlayerInputProcessor) | — |
| 15d | MainControllerAccess/PlayerResourceAccess traits + MainStateType extraction | — |
| 15g | TableData/CourseData cascade unification (CourseData, TrophyData, TableData, TableFolder, TableAccessor) | — |
| 15e | Platform-specific replacements (Twitter4j→bail, AWT clipboard→arboard, PortAudio→cpal, monitors→CoreGraphics FFI) | — |
| 15f | Final stub cleanup (audit 17 crates, remove unused stubs, split rendering_stubs.rs) | — |

## Deferred / Stub Items

**Circular dep stubs (cannot replace):** TextureRegion/Texture in play (isolated in `beatoraja-skin/rendering_stubs.rs`).
**Structural mismatches (resolved):** ~~SongDatabaseAccessor/IRConnection (struct vs trait)~~ → replaced with real traits. ~~BMSPlayerInputProcessor (i32 vs usize)~~ → unified to usize.
**Structural mismatches (resolved):** ~~TableData/TableFolder/TableAccessor (CourseData cascade)~~ → unified CourseData/TrophyData/CourseDataConstraint types, replaced stubs with real imports (Phase 15g).
**Lifecycle stubs (trait-ified):** MainController/PlayerResource stubs remain in downstream crates but now implement `MainControllerAccess`/`PlayerResourceAccess` traits from `beatoraja-types`. MainState uses existing trait in `beatoraja-core`.
**Stub cleanup (P15f):** All unused stubs removed across 9 crates. Rendering stubs isolated in `rendering_stubs.rs`. Remaining `stubs.rs` files contain only: lifecycle stubs (MainController, PlayerResource, Timer), cross-crate re-exports, and Phase 13-deferred items (egui utilities, LibGDX rendering). `beatoraja-audio/stubs.rs` fully emptied.
**External `todo!()`:** LibGDX, ebur128, 7z, MIDI, FLAC/MP3, BGA video, ImGui→egui, LR2 score import, Windows named pipe.
**Platform-specific (resolved P15e):** ~~PortAudio~~ → cpal, ~~Twitter4j~~ → graceful bail, ~~AWT clipboard~~ → arboard, ~~Monitor enumeration~~ → CoreGraphics FFI (macOS).

## Lessons Learned

### Encoding & Platform
- **MS932:** `encoding_rs::SHIFT_JIS.decode(raw_bytes)`. **LR2IR:** Shift_JIS HTTP via `encoding_rs`, XML via `quick-xml`.
- **Platform:** `#[cfg(unix)]`/`#[cfg(windows)]` for Discord IPC, named pipes.

### Borrow Checker
- Parent `this` ref → callback trait (`&mut dyn Trait`). Constructor with sibling → pass primitives.
- LongNote pairing → section-based tracking with index lookups.

### Parallel Agents
- Independent crates → parallel agents. Create workspace `Cargo.toml` + all crate scaffolding BEFORE launching.
- Verify `git status` after — files can be missed. Group by dependency level.

### Stub Management
- Forward stubs in `stubs.rs` per crate. Replace via `pub use real_crate::module::Type;`.
- Add Java-style getters to real types rather than modifying callers.
- Remaining: rendering types, lifecycle types, structural mismatches only.

### Circular Dependencies
- Core cannot import: song, skin, play, select, result, ir, modmenu.
- Solution: `beatoraja-types` crate; core re-exports via `pub use`. BMKeys moved with PlayModeConfig.

### API Incompatibility (Stub → Real)

| Mismatch | Fix |
|---|---|
| `String` vs `Option<String>` | `.unwrap_or_default()` |
| `i32` vs `Mode` | Update callers or adapter methods |
| Struct vs Trait | `Box<dyn Trait>` or `Arc<dyn Trait>` (when Clone needed) |
| Struct vs Enum | Update to enum method calls |
| `set_field(v)` → pub field | Direct assignment |

### Phase-Specific
- **P1:** CommandWord enum → match dispatch. **P2:** switch fallthrough → explicit next-branch call; 16 classes → single `mod.rs`.
- **P3:** MS-ADPCM: `&[u8]` → `Vec<i16>`, static coefficients.
- **P8:** OBS auth: SHA-256 + base64. IRResponse: generic `IRResponse<T>`. IRConnectionManager: `OnceLock` registry. FontAwesome: ~1016 `pub const`. Ghost RLE: 40+ char mappings verbatim.
- **P9:** SkinHeader + items need `#[derive(Clone)]`. **P10:** Custom CRC32 poly `0xEDB88320`, appends `\\\0`. RobustFile: double-write + `sync_all()`.
- **P12:** winit: `create→resumed`, `render→RedrawRequested`, `resize→Resized`, `pause→suspended`, `dispose→CloseRequested`, `ControlFlow::Poll`. CLI: `clap::Parser`; `--replay N`. Deferred: egui launcher, fullscreen (GLFW).
- **P15a:** Moving SongData to `beatoraja-types` required also moving `IpfsInformation` trait (orphan rule: foreign trait on foreign type). Pure interface traits can safely move to low-level crates. Add `full_title(&self) -> String` non-mut helper alongside cached `get_full_title(&mut self) -> &str`. Use `set_path_opt(Option<String>)` / `clear_path()` for `Option` → `String` path migration.
- **P15b:** Moving SkinType: stub had UPPER_SNAKE_CASE + wrong ID mapping (13 variants); real has PascalCase (18 variants). Add `Copy`, `Default`, `Hash` derives. Callers need `as usize` for array indexing after `get_id() -> i32`. Moving GrooveGauge: `create()` depends on `BMSPlayerRule` → extract as free function `create_groove_gauge` in beatoraja-play. Move entire type chain (GaugeModifier, GaugeElementProperty, GaugeProperty, Gauge, GrooveGauge) together since they're tightly coupled. Re-export via `pub use` in original crate modules.
- **P15c:** SongDatabaseAccessor trait needs `: Send` bound when used as `Box<dyn Trait>` inside `Arc<Mutex<...>>`. IRConnection struct→trait: use `Box<dyn IRConnection>` when no Clone needed, `Arc<dyn IRConnection>` when `.clone()` is required (e.g. `IRSendStatus`). `LeaderboardEntry::new_entry_primary_ir` takes owned `IRScoreData` in real (not `&IRScoreData`), callers need `.clone()`. `ClearType` is enum with `.id()` method (not struct with `.id` field). TableData/TableAccessor stubs cannot be replaced without first replacing CourseData (cascade: different field names `song`/`hash`, `String`/`Option<String>`, `f64`/`f32` across ~10 files).
- **P15d:** Lifecycle trait extraction: only include methods whose param/return types exist in `beatoraja-types` (Config, PlayerConfig, ScoreData, SongData, etc.); methods needing types from other crates (BMSPlayerInputProcessor, SystemSoundManager, IRStatus) stay as inherent methods on local stubs. When trait method names conflict with existing inherent methods, rename inherent method (e.g. `get_player_config` → `get_player_config_local`). MainStateAccess trait deferred — existing `MainState` trait in core already covers the interface; downstream stubs have too-divergent APIs. `MainStateType` moves from core to types like other shared enums.
- **P15g:** CourseData cascade: once CourseData/TrophyData/CourseDataConstraint stubs are replaced with real types from `beatoraja-types`, TableData/TableFolder/TableAccessor stubs can be replaced with imports from `beatoraja-core`. Key changes: `TableAccessor` trait needs `: Send + Sync` bounds for `Box<dyn TableAccessor>`. Real `TableData::get_url()` returns `&str` (not `Option<&str>`); use `get_url_opt()` for callers that need `Option`. `TrophyData` rates changed `f64` → `f32`, update arithmetic in `grade_bar.rs`. `BMSSearchAccessor` trait impl: `read()` returns `Option<TableData>`, `write()` takes `&mut TableData`.
- **P15e:** Platform-specific replacements: Twitter4j has no Rust equivalent — replace `todo!()` with `anyhow::bail!()` to avoid runtime panics. AWT clipboard → `arboard` crate (image clipboard needs `image` crate for PNG decoding). PortAudio → `cpal` crate (`default_host().output_devices()`). winit 0.30 `available_monitors()` only on `ActiveEventLoop` (not `EventLoop`) — use CoreGraphics FFI (`CGGetActiveDisplayList`/`CGDisplayBounds`) on macOS; proper winit enumeration deferred to Phase 13 egui integration. Rust 2024 edition requires `unsafe extern "C"` blocks.
- **P15f:** Stub cleanup: audit agents frequently flag items as "unused" when they're actually referenced — always verify with `cargo check` after removal. Cross-crate re-exports (`beatoraja_skin::stubs::Color` used by select/result) require checking downstream crates, not just local usage. Split rendering stubs into `rendering_stubs.rs` with `pub use crate::rendering_stubs::*` in `stubs.rs` for backward compatibility — avoids updating 50+ import statements. Lifecycle stubs (MainController, PlayerResource, Timer, etc.) remain in `stubs.rs` as they'll be replaced when real cross-crate wiring is complete. `beatoraja-audio/stubs.rs` was fully emptied (all items unused). `beatoraja-launcher` utility stubs (file dialogs, URL opener) confirmed deferred to Phase 13 egui integration.
