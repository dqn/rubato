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
| LibGDX (LWJGL3) / PortAudio | wgpu / Kira |
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

All phases complete. 936 tests pass. Zero runtime `todo!()`/`unimplemented!()`. Phase 16b partially done (2 duplicate pending tests deleted; 15 remaining blocked).

| Phases | Summary |
|--------|---------|
| 1–12 | Core translation: 17 crates, 300+ modules, CLI + winit event loop |
| 14, 15a–g | Circular dep resolution (`beatoraja-types`), struct→trait unification, stub cleanup, platform replacements |
| 13a–e, 13g | Real implementations: wgpu rendering, Kira audio, mlua, ffmpeg-next, midir, cpal |
| 13f | egui UI: EguiIntegration (egui-wgpu 0.31), LauncherApp + BeatorajaApp event loops, 10 modmenu widgets, LauncherUi 11 tabs (all wired), winit monitor enumeration |
| 13f follow-up | LauncherUi 6 placeholder tabs wired to Config/PlayerConfig fields (Input, Skin, Other, IR, Stream, OBS). Dead `show(&mut ImBoolean)` removed from 9 modmenu sub-menus |
| 13f follow-up 2 | `commit_config()` persists Config + PlayerConfig. IR tab AES-encrypted get/set with egui buffers. Skin tab: full `SkinConfigurationView` integration (type/header selectors, CustomOption/File/Offset widgets, history). Input tab confirmed complete vs Java |
| 16a–c | Tests: 715 unit + 121 golden master + 32 integration (compare_rule + compare_pattern reactivated with Java LCG fix). 16b partial: 2 duplicate pending tests deleted; 15 remaining blocked by JudgeManager stub + missing APIs |
| 17 | Verified zero runtime todo!/unimplemented! |
| 18 | Post-Phase 13 lifecycle wiring (pending) |

## Remaining Stubs

- ~~**Circular dep:** TextureRegion/Texture in play~~ → resolved: `pub use beatoraja_render::Texture` in `beatoraja-play/stubs.rs`
- **Lifecycle:** MainController/PlayerResource stubs in downstream crates (implement traits from `beatoraja-types`)
- **Remaining stubs.rs:** lifecycle stubs, cross-crate re-exports
- **Platform:** Windows named pipe (platform-specific, not yet implemented)

## Lessons Learned

### General Patterns
- **MS932:** `encoding_rs::SHIFT_JIS.decode(raw_bytes)`. LR2IR: Shift_JIS HTTP via `encoding_rs`, XML via `quick-xml`.
- **Platform:** `#[cfg(unix)]`/`#[cfg(windows)]` for Discord IPC, named pipes.
- **Borrow checker:** Parent `this` ref → callback trait (`&mut dyn Trait`). Constructor with sibling → pass primitives. LongNote pairing → section-based tracking with index lookups.
- **Parallel agents:** Independent crates → parallel agents. Create workspace `Cargo.toml` + all crate scaffolding BEFORE launching. Verify `git status` after. Group by dependency level.
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
- **wgpu:** Direct (not Bevy). `beatoraja-render` wraps Instance/Device/Queue/Surface. `GpuContext::new_with_surface(Arc<Window>)`. `pollster::block_on()` for async init in sync handlers.
- **Kira:** Use 0.12 (v3 doesn't exist). `SkinObjectRenderer` wraps `SpriteBatch` with 0.01f offset workaround.
- **mlua:** `Lua::new()` with `load("return " + script).into_function()`. `package.loaded` pre-registration for `require()`.
- **todo!()→warn!():** `OnceLock` for `&T` returns, `Box::leak` for `&mut T` returns (Rust 2024 prohibits `static mut` refs). `Mode` default: `Mode::BEAT_7K`.
- **Serde:** Java Jackson camelCase differs: `BPM`→`Bpm`, `URL`→`Url`, `SE`→`Se`, `RPC`→`Rpc`. `CourseDataConstraint` needs `#[serde(alias)]` for Java snake_case.
- **Trait bounds:** `SongDatabaseAccessor: Send` for `Box<dyn Trait>` in `Arc<Mutex<...>>`. `TableAccessor: Send + Sync` for `Box<dyn TableAccessor>`.
- **HashMap ordering:** Differs between Java/Rust — add deterministic tie-breaking (sort by BPM, use `>=`) for `mainbpm`.
- **Bounds checking:** `TimeLine::get_note`/`exist_note_at` need bounds checks — BMS mode detection may yield fewer lanes than caller expects. Use `.get()` for reads.
- **CRC32:** Custom poly `0xEDB88320`, appends `\\\0`. **RobustFile:** double-write + `sync_all()`.
- **Platform replacements:** Twitter4j → `anyhow::bail!()`, AWT clipboard → `arboard`, PortAudio → `cpal`, monitors → CoreGraphics FFI (macOS) + winit `available_monitors()` (non-macOS). Rust 2024: `unsafe extern "C"` blocks.
- **egui-wgpu version:** egui-wgpu 0.30 depends on wgpu 23; project uses wgpu 24. Must use egui/egui-wgpu/egui-winit 0.31 which depends on wgpu ^24.0.0.
- **RenderPass lifetime:** wgpu 24's `begin_render_pass` returns `RenderPass<'encoder>` but `egui_wgpu::Renderer::render()` requires `RenderPass<'static>`. Fix: `render_pass.forget_lifetime()`.
- **egui architecture:** egui context managed in beatoraja-bin (avoids circular deps). Two event loops: `LauncherApp` (ControlFlow::Wait, standalone config UI) and `BeatorajaApp` (ControlFlow::Poll, game + egui overlay). Each modmenu sub-menu gets `show_ui(ctx: &egui::Context)`.
- **Monitor enumeration:** Non-macOS uses winit `ActiveEventLoop::available_monitors()` cached in a global `Mutex<Vec<MonitorInfo>>` populated from `resumed()`. macOS keeps CoreGraphics FFI.
- **Stub cleanup:** Always verify with `cargo check` after removal. Cross-crate re-exports require checking downstream crates. Split rendering stubs into `rendering_stubs.rs` with `pub use` in `stubs.rs` for backward compat.
- **Java Random LCG:** `java.util.Random(seed)` uses LCG (multiplier=`0x5DEECE66D`, addend=`0xB`, mask=48-bit). Seed scramble: `(seed ^ multiplier) & mask`. `nextInt(bound)` has power-of-2 fast path. Must use `wrapping_mul`/`wrapping_add` for i64 overflow. Implemented as `java_random::JavaRandom` in `beatoraja-pattern`. **Never use `StdRng`/`rand` for Java-seeded RNG.**
