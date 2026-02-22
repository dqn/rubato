# lr2oraja Rust Porting — Mechanical Line-by-Line Translation

lr2oraja (beatoraja fork, Java 313 files / 72k+ lines) → Rust. Source: `./lr2oraja-java`.

## Rules

- Workflow: `Read Java → Write Rust → Test → Next`. Copy Java verbatim, refactor ONLY after ALL tests pass.
- Translate one method → test immediately — green before moving on.
- Golden Master: export Java values as JSON, compare with Rust. Tolerance: ±2μs.
- Preserve ALL branch/loop/fallthrough structure. Copy constants/magic numbers AS-IS.
- Explicit type conversions — every implicit Java cast → explicit Rust cast.
- After completing a phase/task, update TODO.md and AGENTS.md.
- Worktree isolation: **always merge worktree branches before sending shutdown requests**.
- Deferred items: always tag with `→ **Phase XX**`. At phase completion, audit all deferred items. When creating a new phase, grep TODO.md for `→ **Phase {number}**`.

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

`lr2oraja-java/` (read-only) · `lr2oraja-rust/` (Cargo workspace: `crates/`, `golden-master/`, `test-bms/`)

## Key Invariants

- Timing: i64 microseconds. JavaRandom LCG in `beatoraja-pattern` (**never** `StdRng`/`rand`). LR2 MT19937. LR2 judge: pure integer arithmetic. LongNote: index-based.

## Testing

- **Golden Master:** `just golden-master-gen`. Fixtures: `filename.ext.json`.
- **TDD:** Red-Green-Refactor. **ast-compare:** `just ast-map` / `just ast-compare` / `just ast-constants` / `just ast-full`.

## Status

**1739 tests, 22 ignored (RenderSnapshot — Phase 26).** Phases 1–25d complete: 17 crates, 300+ modules. Zero `todo!()`/`unimplemented!()`. Zero clippy warnings.

## Remaining Stubs (~900 lines / 10 files)

1. **API-incompatible** (→ Phase 29a): result/decide/select/modmenu rendering stubs, Property traits
2. **Intentional** (permanent): Twitter4j → `bail!()` (~155 lines)

## Lessons Learned

- **Encoding:** `encoding_rs::SHIFT_JIS` for MS932. **Serde:** `BPM`→`Bpm`, `URL`→`Url`, `#[serde(alias)]`.
- **Borrow checker:** `&mut` conflicts → scoped block. Self-reference → `Option::take()` + put-back. Parent ref → callback trait.
- **Stubs:** `stubs.rs` per crate → replace via `pub use`. Always `cargo check` after removal.
- **Circular deps:** `beatoraja-types` for shared types. Core cannot import: song, skin, play, select, result, ir, modmenu. `StateFactory`/`SkinDrawable` traits in core, impl in downstream crates.
- **API mismatch:** `String`↔`Option<String>` → `.unwrap_or_default()`; `&self`↔`&mut self` → scoped block / `Box::leak`.
- **Libraries:** winit (`resumed`/`RedrawRequested`/`Poll`), wgpu (direct, `pollster::block_on()`), Kira 0.12, mlua (`load("return "+s)`), egui (`RenderPass::forget_lifetime()`).
- **Patterns:** `OnceLock` for `&T`, `Box::leak` for `&mut T`. CRC32 poly `0xEDB88320` + `\\\0`. RobustFile: double-write + `sync_all()`. BRD replay: gzip JSON. PlayerResource: trait (32 methods) + `NullPlayerResource`.
