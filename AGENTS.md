# lr2oraja Rust Porting

beatoraja fork (Java 313 files / 72k+ lines) → Rust. 26 crates, 167k lines. Source: `./lr2oraja-java`.

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

**3230 tests.** Phases 1–62 complete + post-62 stub audit. Zero clippy warnings. Zero regressions.
**Migration audit**: 100% method resolution (4,279/4,279). 0 missing. 0 constant mismatches. Gap: 0.
**ast-compare**: 2,235 methods ignored (789 patterns). Method-level ignore via `.ast-compare-method-ignore`.
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
- **Post-62e**: Select crate stubs replaced with real beatoraja-skin types (SkinImage, SkinNumber, SkinText trait, SkinObjectRenderer, SkinRegion→Rectangle alias). GlyphAtlas + BitmapFont::draw() implemented with row-packing atlas and SpriteBatch glyph rendering. MessageRenderer wired with SpriteBatch + alpha pulsing animation.

### Permanent Stubs (intentionally unimplemented)

- **Twitter4j** (`beatoraja-external`): ~446 lines, `bail!()` — API deprecated
- **ShortDirectPCM** (`beatoraja-audio`): Java-specific DirectBuffer — unnecessary in Rust
- **JavaFX find_parent_by_class_simple_name** (`beatoraja-launcher`): No egui equivalent
- **randomtrainer.dat** (`beatoraja-modmenu`): Binary resource from Java, uses empty HashMap fallback

## Lessons Learned

- **Encoding:** `encoding_rs::SHIFT_JIS` for MS932. **Serde:** `BPM`→`Bpm`, `URL`→`Url`, `#[serde(alias)]`.
- **Borrow checker:** `&mut` conflicts → scoped block. Self-ref → `Option::take()`. Parent ref → callback trait.
- **Stubs:** `stubs.rs` per crate → replace via `pub use`. Always `cargo check` after removal.
- **Circular deps:** `beatoraja-types` for shared types. Core cannot import: song, skin, play, select, result, ir, modmenu.
- **Lua→JSON coercion:** 3-layer: numbers→strings, float→int truncation, empty `{}`→remove.
- **Bar Clone:** `Box<dyn Trait>` blocks Clone → use `Arc<dyn Trait>` for shared trait objects.
- **Property delegate pattern:** `integer_value(id)` / `float_value(id)` / `boolean_value(id)` on MainState — skin property factories delegate via ID lookup.
- **ast-compare false positives:** ~88% of "missing" methods are architectural redesigns. Always verify Java↔Rust manually before implementing.
- **ast-compare method-level ignore:** `.ast-compare-method-ignore` supports `ClassName.methodName` (exact) and `ClassName.*` (wildcard). Run `just ast-map` to use.
- **Java float→int→byte truncation:** Use `as i32 as i8` in Rust (via i32 to get truncation). Direct `as i8` saturates since Rust 1.45.

## Landing the Plane (Session Completion)

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Sync beads** - `bd sync`
5. **Clean up** - Clear stashes, prune remote branches
6. **Hand off** - Provide context for next session
