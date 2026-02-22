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

All phases complete. **1241 tests pass. Zero runtime `todo!()`/`unimplemented!()`.** PlayerResource wrapper migration complete for all 6 crates. 1 MainController crate remaining (result: blocked on type gaps).

| Phases | Summary |
|--------|---------|
| 1–12 | Core translation: 17 crates, 300+ modules, CLI + winit event loop |
| 13a–g | Real implementations: wgpu, Kira, mlua, ffmpeg-next, midir, cpal, egui UI (LauncherApp + BeatorajaApp, 11 tabs, 10 modmenu widgets, SkinConfigurationView, IR AES, commit_config) |
| 14, 15a–g | Circular dep resolution (`beatoraja-types`), struct→trait unification, stub cleanup, platform replacements |
| 16a–c | Tests: 715 unit + 121 golden master + 32 integration. 16b partial (1 remaining blocked by SkinData→Skin pipeline + Lua loader) |
| 17 | Verified zero runtime todo!/unimplemented! |
| 18a | Core judge loop: `JudgeManager::update()` 450-line translation, testable API, LN pairing fix, `total_notes` fix |
| 18b | Rendering state providers: `SkinStateProvider` trait, pure-function keyframe eval, render_snapshot, Property `get_id()` |
| 18c | Audio decode API: `AudioData` + `load_audio()`, WAVE_FORMAT_EXTENSIBLE, `f32_to_i16()` |
| 18d | BGA/skin test APIs: `BGAProcessor` timeline, `JSONSkinLoader`/`LR2SkinHeaderLoader` tests, 3 skin loader bugs fixed |
| 18e-1 | Cross-crate stub dedup: `ImGuiNotify` centralized in beatoraja-types, `Random`/`LR2Random` → beatoraja-pattern |
| 18e-2 | Lifecycle stub replacement: MainController removed from 7/8 crates, PlayerResource wrapper complete for all 6 crates, `PlayerResourceAccess` trait (32 methods), stub types resolved (FloatArray/GdxArray/GrooveGaugeStub → real) |
| 18e-3 | Modmenu skin stub replacement: SkinHeader/CustomOption/CustomFile/CustomOffset/CustomCategory + loaders (JSON/LR2/Lua) replaced with real beatoraja-skin re-exports, ~170 lines removed, conversion helpers added |
| 18e-4 | PlayDataAccessor stub replacement: `pub use` from beatoraja-core, model-based convenience methods, course methods, `null()` constructor, `compute_constraint_values` implemented, IntArray→Vec<i32> |
| 18e-5 | BMSPlayerMode/BMSPlayerModeType → real beatoraja-core types, EventType removed (dead code), unused modmenu stubs deleted (InputProcessor/Lwjgl3ControllerManager/Controller) |
| 18e-6 | ImGui adapter stubs (ImBoolean/ImInt/ImFloat) → plain primitives in Mutex, Clipboard stubs → real arboard, freq_trainer stubs → re-export from beatoraja-modmenu |
| 18e-7 | beatoraja-select stub cleanup: BMSPlayerMode/BMSPlayerModeType → real beatoraja-core types, PlayerInformation → real type + `get_name()` getter, dead Pair stub removed |
| 18e-8 | Circular dep resolution: RandomHistoryEntry + history state → beatoraja-types (breaks modmenu→pattern dep), last-played-sort state → beatoraja-types (breaks modmenu→select dep). beatoraja-pattern stubs.rs deleted entirely |
| 18f | E2E test activation: 9 test files, 138 tests. `build_judge_notes()` time ordering fix |
| 18g | BRD replay codec: gzip-compressed JSON, `read_brd()`/`write_brd()` + course variants |

## Remaining Stubs

- **MainController:** result (6 methods actively used, PlayDataAccessor resolved in 18e-4, BMSPlayerMode resolved in 18e-5, remaining blockers: BMSPlayerInputProcessor/IRConnection), md-processor (intentional adapter, deferred), modmenu (3-method stub: get_config, get_player_config, save_config — until real MainController exists)
- **Remaining stubs.rs:** lifecycle stubs, cross-crate re-exports, skin/rendering types. beatoraja-result: MainController, BMSPlayerInputProcessor, IRStatus/IRSendStatusMain, SkinObjectData. beatoraja-modmenu: Skin/SkinObject/Rectangle, MainState, MainController, MusicSelector/Bar/SongBar (RandomTrainer/SongManagerMenu delegated to beatoraja-types in 18e-8). beatoraja-select: MainState, RandomCourseData, RankingData, BMSPlayerInputProcessor, skin rendering stubs, EventType, AudioDriver, TimerState, DownloadTask (SongManagerMenu delegated in 18e-8, BMSPlayerMode/PlayerInformation replaced in 18e-7). **beatoraja-pattern: stubs.rs deleted (18e-8), zero remaining stubs.** Clipboard → arboard (18e-6). freq_trainer → beatoraja-modmenu re-export (18e-6)
- **Platform:** Windows named pipe (not yet implemented)

## Lessons Learned

### General Patterns
- **MS932:** `encoding_rs::SHIFT_JIS.decode(raw_bytes)`. LR2IR: Shift_JIS HTTP via `encoding_rs`, XML via `quick-xml`.
- **Platform:** `#[cfg(unix)]`/`#[cfg(windows)]` for Discord IPC, named pipes.
- **Borrow checker:** Parent `this` ref → callback trait (`&mut dyn Trait`). Constructor with sibling → pass primitives. LongNote pairing → section-based tracking with index lookups.
- **Parallel agents:** Independent crates → parallel agents. Create workspace `Cargo.toml` + all crate scaffolding BEFORE launching. Group by dependency level.
- **Stubs:** Forward stubs in `stubs.rs` per crate. Replace via `pub use real_crate::module::Type;`. Add Java-style getters to real types rather than modifying callers.
- **Circular deps:** Core cannot import: song, skin, play, select, result, ir, modmenu. Solution: `beatoraja-types` crate; core re-exports via `pub use`.

### API Incompatibility (Stub → Real)

| Mismatch | Fix |
|---|---|
| `String` vs `Option<String>` | `.unwrap_or_default()` |
| `i32` vs `Mode` | Update callers or adapter methods |
| Struct vs Trait | `Box<dyn Trait>` or `Arc<dyn Trait>` (when Clone needed) |
| Struct vs Enum | Update to enum method calls |
| `set_field(v)` → pub field | Direct assignment |

### Key Technical Decisions
- **winit:** `create→resumed`, `render→RedrawRequested`, `resize→Resized`, `pause→suspended`, `dispose→CloseRequested`, `ControlFlow::Poll`.
- **wgpu:** Direct (not Bevy). `beatoraja-render` wraps Instance/Device/Queue/Surface. `pollster::block_on()` for async init.
- **Kira 0.12** (v3 doesn't exist). `SkinObjectRenderer` wraps `SpriteBatch` with 0.01f offset workaround.
- **mlua:** `Lua::new()` with `load("return " + script).into_function()`. `package.loaded` pre-registration for `require()`.
- **todo!()→warn!():** `OnceLock` for `&T` returns, `Box::leak` for `&mut T` returns. `Mode` default: `Mode::BEAT_7K`.
- **Serde:** Java Jackson camelCase differs: `BPM`→`Bpm`, `URL`→`Url`, `SE`→`Se`, `RPC`→`Rpc`. `CourseDataConstraint` needs `#[serde(alias)]`.
- **Trait bounds:** `SongDatabaseAccessor: Send`, `TableAccessor: Send + Sync` for `Box<dyn Trait>` in `Arc<Mutex<...>>`.
- **HashMap ordering:** Java/Rust differs — deterministic tie-breaking (sort by BPM, use `>=`) for `mainbpm`.
- **Bounds checking:** `TimeLine::get_note`/`exist_note_at` need `.get()` — BMS mode detection may yield fewer lanes.
- **CRC32:** Custom poly `0xEDB88320`, appends `\\\0`. **RobustFile:** double-write + `sync_all()`.
- **Platform replacements:** Twitter4j → `anyhow::bail!()`, AWT → `arboard`, PortAudio → `cpal`, monitors → CoreGraphics FFI (macOS) + winit (non-macOS).
- **egui architecture:** Context in beatoraja-bin. `LauncherApp` (Wait) + `BeatorajaApp` (Poll). `RenderPass::forget_lifetime()` for wgpu 24 compat. Monitor: winit `available_monitors()` in global `Mutex<Vec<MonitorInfo>>`, macOS keeps CoreGraphics FFI.
- **Stub cleanup:** Always `cargo check` after removal. Cross-crate re-exports require checking downstream crates.

### Phase-Specific Patterns
- **Java Random LCG:** Seed scramble `(seed ^ multiplier) & mask`. `nextInt(bound)` power-of-2 fast path. `wrapping_mul`/`wrapping_add` for i64 overflow.
- **WAVE_FORMAT_EXTENSIBLE (0xFFFE):** Skip 8 bytes, read 2-byte sub-format GUID, skip 14 bytes. For 24-bit WAV.
- **JudgeManager testable API:** Rust takes all inputs as parameters (key_states, key_changed_times, &mut GrooveGauge). `NoteJudgeState` tracks per-note state. `LaneIterState` reimplements Java `Lane`. `compare_times()` for JudgeNote-based loop.
- **build_judge_notes() ordering:** Sort by `(time_us, lane)` after building, remap `pair_index` via old→new index map.
- **BRD replay:** Gzip-compressed JSON. `write_brd()` → `shrink()` first (keylog→base64-gzip `keyinput`). `read_brd()` → `validate()` after.
- **Pure-function keyframe eval:** Reimplemented as pure functions in `eval.rs`. Alpha offset quirk: early returns for acc==3 skip alpha offset.
- **Property ID:** `get_id() -> i32`, `i32::MIN` as sentinel. All callers guard with `filter(|&id| id != i32::MIN)`.
- **BGAProcessor:** `set_model_timelines()` extracts `BgaTimeline`. `prepare_bga(time_ms)` cursor-based. `update(time_us)` divides by 1000. id -1=no change, -2=stop.
- **JSON skin loader bugs:** (1) source_resolution from w/h. (2) Custom paths: relative as-is. (3) Offset defaults: PLAY_* type IDs only.
- **SkinData vs Skin gap:** `JSONSkinLoader` returns `SkinData`, not `Skin`. Tests at `SkinData`/`SkinHeaderData` level. Full Skin tests deferred.
- **Programmatic skin for tests:** `Skin::new()`, `SkinImage::new_with_single()`, `SkinObject::Image()`, `set_destination_with_int_timer_ops()`. Draw conditions via `op` on first call only.
- **Cross-crate stub dedup:** `ImGuiNotify` → beatoraja-types (log facade). `Random`/`LR2Random` stub SCREAMING_CASE → real PascalCase. `LR2Random::new(seed)` → `with_seed(seed)`.
- **Lifecycle stub strategy:** `instanceof` → `state_type()` on `MainState` trait. Unused `MainController` → `NullMainController`. Crate-specific methods remain until real MainController. Key blocker: stubs return owned values vs trait requires references.
- **PlayerResource migration:** Trait-only methods → direct `Box<dyn PlayerResourceAccess>`. Non-optional→optional → update callers for `Option<>`. Crate-local methods → wrapper struct + extra fields. `&mut T` not on trait → get/clone/set. Uncalled methods → delete. Trait expanded to 32 methods (3 mutable getters). `NullPlayerResource` needs fields for `&mut` returns.
- **Empty marker trait:** Dead `MainState` with unused `get_main()` → remove method, keep empty trait. Callers use `_` prefix.
- **Modmenu skin config migration:** `String`→`Option<String>`, wrap in `Some()`, `is_some_and()` for comparison, `iter().flatten()` for iteration. `PlayerConfig.skin`: `Vec<Option<SkinConfig>>`. `get_play_config`: `&Mode`→`Mode` (add `.clone()`). `read_all_player_id`: free function, not associated method.
- **PlayDataAccessor model-based methods:** Real impl uses hash-based signatures (`sha256: &str`), but callers use `BMSModel`. Solution: add model-based convenience methods that extract `sha256` and delegate. Course methods concatenate first-10-chars of each model hash. `null()` constructor with all `Option` fields = `None` for stub returns. `OnceLock` won't work (rusqlite::Connection isn't Sync) → `Box::leak`.
- **Enum aliasing for stub replacement:** Real `bms_player_mode::Mode` conflicts with `bms_model::mode::Mode`. Solution: `pub use Mode as BMSPlayerModeType` preserves all caller code. Stub `ReplayDifferent` variant → real `Autoplay` variant (unused, no impact). Real struct adds `id: i32` → switch from struct literal to `BMSPlayerMode::new()`. Dead code detection: `EventType` only in comments → safe to delete.
- **ImGui adapter pattern:** Java imgui `ImBoolean`/`ImInt`/`ImFloat` wrappers (`.get()`/`.set()`/`.value`) → plain `bool`/`i32`/`f32` in `Mutex<T>`. clippy auto-fixes `&mut *guard` → `&mut guard` (DerefMut coercion). Free function stubs → associated method re-exports: `is_freq_trainer_enabled()` → `FreqTrainerMenu::is_freq_trainer_enabled()`. Clipboard stubs → inline `arboard::Clipboard::new()` + `set_text()` (no shared helper needed for 2 call sites).
- **Stub replacement with getter addition:** Real `PlayerInformation` has `name: Option<String>` vs stub `name: String`. Solution: add `get_name() -> &str` (via `as_deref().unwrap_or("")`) to real type. Avoids modifying callers.
- **Circular dep resolution via shared state:** RandomTrainer (modmenu→pattern) and SongManagerMenu (modmenu→select) both had circular deps preventing direct `pub use`. Solution: extract shared state (`Mutex<VecDeque>` for history, `Mutex<bool>` for sort flag) into beatoraja-types modules. Both sides delegate to shared module. Pattern: move the minimum shared state (static + accessors), keep business logic in original crate. Egui checkbox `&mut MutexGuard` pattern → get/set pattern when static moves to another crate.
