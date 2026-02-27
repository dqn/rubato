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

- **Test runner:** `just test` (excludes slow render snapshot tests) or `just test-all` (full).
- **Golden Master:** `just golden-master-gen`. Fixtures: `filename.ext.json`.
- **TDD:** Red-Green-Refactor. **ast-compare:** `just ast-map` / `just ast-compare` / `just ast-constants` / `just ast-full`.

## Status

**2346 tests, 16 ignored (4 env + 12 bug-doc).** Phases 1–43 complete. Zero clippy warnings.
**Migration audit**: 93.97% method resolution (4,021/4,279). 0 constant mismatches. 3 Rust-side regressions.

## Remaining Stubs (~2,746 lines across 10 stubs.rs + ~186 inline markers)

| Crate | stubs.rs | Markers | Key Blockers |
|-------|:--------:|:-------:|-------------|
| beatoraja-launcher | 527 | 13 | Skin header loading, async BMS DB |
| beatoraja-result | 494 | 27 | CourseResult MainState non-functional, IR thread |
| beatoraja-external | 446 | 4 | Permanent (`bail!()`, Twitter API deprecated) |
| beatoraja-skin | 389 | 44 | Timer frozen, FloatProperty all 0.0, Lua 19/28 missing |
| beatoraja-select | 278 | 55 | 7 bar get_children(), read_chart blocked on PlayerResource |
| beatoraja-modmenu | 205 | — | Needs SkinWidget rewrite (Phase 40) |
| beatoraja-decide | 204 | — | AudioProcessor/Skin stubs |
| beatoraja-audio | 190 | — | GdxAudioDeviceDriver no-op, BMSLoudnessAnalyzer hardcoded |
| beatoraja-input | 115 | — | MouseScratchInput position hardcoded |
| beatoraja-types | 88 | — | 7 resolved re-exports, 1 partial (BarSorter) |
| beatoraja-core | 1 | 33 | PlayerResource.loadBMSModel, load_skin, exit, save_config |

### Critical Gaps (7)

1. PlayerResource.loadBMSModel() — cannot load BMS files
2. MainState.load_skin() — all screens blank
3. PlayerResource.SongData type mismatch — get_songdata() always None
4. read_chart/read_course/read_random_course — select→play blocked
5. CourseResult MainState non-functional — course results unplayable
6. FloatPropertyFactory ALL stubs — gauges/covers/rates invisible
7. SkinTextFont.draw_with_offset() — TrueType text invisible

### Regressions (3)

1. **RandomizerBase uses StdRng** (HIGH) — breaks replay/pattern reproducibility
2. BytePCM float saturation (Medium) — clipping differs from Java
3. BytePCM negative overflow (Medium) — same issue, negative range

## Lessons Learned

- **Encoding:** `encoding_rs::SHIFT_JIS` for MS932. **Serde:** `BPM`→`Bpm`, `URL`→`Url`, `#[serde(alias)]`.
- **Borrow checker:** `&mut` conflicts → scoped block. Self-ref → `Option::take()`. Parent ref → callback trait.
- **Stubs:** `stubs.rs` per crate → replace via `pub use`. Always `cargo check` after removal.
- **Circular deps:** `beatoraja-types` for shared types. Core cannot import: song, skin, play, select, result, ir, modmenu.
- **Lua→JSON coercion:** 3-layer: numbers→strings, float→int truncation, empty `{}`→remove.
