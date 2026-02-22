# lr2oraja Rust Porting

beatoraja fork (Java 313 files / 72k+ lines) → Rust. 27 crates, 122k lines. Source: `./lr2oraja-java`.

## Rules

- Workflow: `Read Java → Write Rust → Test → Next`. Copy Java verbatim, refactor ONLY after ALL tests pass.
- Translate one method → test immediately — green before moving on.
- Golden Master: export Java values as JSON, compare with Rust. Tolerance: ±2μs.
- Preserve ALL branch/loop/fallthrough structure. Copy constants/magic numbers AS-IS.
- Explicit type conversions — every implicit Java cast → explicit Rust cast.
- After completing a phase/task, update TODO.md and AGENTS.md.
- Worktree isolation: **always merge worktree branches before sending shutdown requests**.
- Deferred items: always tag with `→ **Phase XX**`. At phase completion, audit all deferred items.

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
lr2oraja-java/       # Java source (read-only)
lr2oraja-rust/       # Cargo workspace
  crates/
    bms-model        # BMS/BME/BML parser + model
    bms-table        # Difficulty table parser
    beatoraja-types  # Shared types (circular dep breaker)
    beatoraja-common # Config, DB schema, utilities
    beatoraja-pattern    # Note pattern (JavaRandom LCG)
    beatoraja-audio      # Audio (Kira 0.12)
    beatoraja-input      # Keyboard/controller input
    beatoraja-controller # gilrs controller manager
    beatoraja-render     # Rendering (wgpu)
    beatoraja-skin       # Skin loading/layout
    beatoraja-song       # Song DB (rusqlite)
    beatoraja-core       # State machine, main loop
    beatoraja-play       # Play state (gameplay)
    beatoraja-select     # Song select state
    beatoraja-decide     # Song decide state
    beatoraja-result     # Result state
    beatoraja-modmenu    # Mod menu state
    beatoraja-ir         # Internet ranking
    beatoraja-external   # Twitter, clipboard
    beatoraja-obs        # OBS WebSocket
    beatoraja-stream     # Streaming
    beatoraja-launcher   # Launcher UI (egui)
    beatoraja-system     # Platform utilities
    beatoraja-bin        # Entry point
    discord-rpc          # Discord Rich Presence
    md-processor         # Markdown processing
    ast-compare          # Test: AST Java↔Rust comparison
  golden-master/   # Golden Master test infra
  test-bms/        # Test BMS files
```

## Key Invariants

- Timing: i64 microseconds. JavaRandom LCG in `beatoraja-pattern` (**never** `StdRng`/`rand`). LR2 MT19937. LR2 judge: pure integer arithmetic. LongNote: index-based.

## Testing

- **Golden Master:** `just golden-master-gen`. Fixtures: `filename.ext.json`.
- **TDD:** Red-Green-Refactor. **ast-compare:** `just ast-map` / `just ast-compare` / `just ast-constants` / `just ast-full`.

## Status

**1780 tests, 22 ignored.** Phases 1–26b, 26d complete. Zero clippy warnings. Phase 26c next.

## Remaining Stubs (10 `stubs.rs` files, ~2,600 lines)

| Crate | Lines | Category | Target |
|-------|------:|----------|--------|
| beatoraja-types | 549 | Shared type stubs | Phase 29a |
| beatoraja-external | 446 | Twitter4j (`bail!()`) + clipboard | Permanent / 29a |
| beatoraja-result | 388 | Rendering stubs | Phase 29a |
| beatoraja-select | 317 | Rendering stubs | Phase 29a |
| beatoraja-launcher | 314 | Egui integration | Phase 26+ |
| beatoraja-skin | 287 | Skin pipeline | Phase 26c |
| beatoraja-modmenu | 159 | Rendering stubs | Phase 29a |
| beatoraja-decide | 108 | Rendering stubs | Phase 29a |
| beatoraja-input | 44 | Input device | Phase 28a |
| beatoraja-core | 1 | (empty) | — |

**Categories:** Rendering (→29a) · Skin pipeline (→26b) · Platform/input (→28) · Twitter4j (permanent `bail!()`)

## Lessons Learned

- **Encoding:** `encoding_rs::SHIFT_JIS` for MS932. **Serde:** `BPM`→`Bpm`, `URL`→`Url`, `#[serde(alias)]`.
- **Borrow checker:** `&mut` conflicts → scoped block. Self-reference → `Option::take()` + put-back. Parent ref → callback trait.
- **Stubs:** `stubs.rs` per crate → replace via `pub use`. Always `cargo check` after removal.
- **Circular deps:** `beatoraja-types` for shared types. Core cannot import: song, skin, play, select, result, ir, modmenu. `StateFactory`/`SkinDrawable` traits in core, impl in downstream crates.
- **API mismatch:** `String`↔`Option<String>` → `.unwrap_or_default()`; `&self`↔`&mut self` → scoped block / `Box::leak`.
- **Libraries:** winit (`resumed`/`RedrawRequested`/`Poll`), wgpu (direct, `pollster::block_on()`), Kira 0.12, mlua (`load("return "+s)`), egui (`RenderPass::forget_lifetime()`).
- **Patterns:** `OnceLock` for `&T`, `Box::leak` for `&mut T`. CRC32 poly `0xEDB88320` + `\\\0`. RobustFile: double-write + `sync_all()`. BRD replay: gzip JSON. PlayerResource: trait (32 methods) + `NullPlayerResource`.
