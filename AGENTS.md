# lr2oraja Rust Porting Project — Mechanical Line-by-Line Translation

## Overview

lr2oraja (beatoraja fork, Java 313 files / 72,000+ lines) to Rust.
All features including peripherals (Launcher, ModMenu, OBS, Discord RPC, Downloader) are in scope.

**CRITICAL: This is a FRESH START. All previous Rust code has been discarded.**

## Absolute Rules

### No Investigation, No Planning — Just Translate

- **NEVER** spend time exploring, investigating, or analyzing Java code structure before translating.
- **NEVER** enter plan mode or create plans for translation work.
- **NEVER** use Explore agents or research agents to "understand" the codebase.
- **Just read each Java file and translate it to Rust immediately.** The translation is mechanical — no design decisions needed.
- The workflow is: `Read Java file → Write Rust file → Test → Next file`.

### Prohibition on Past History

- **NEVER** read, reference, or consult any previous implementation, plans, or notes.
- **NEVER** use `git log`, `git show`, or browse old commits for implementation guidance.
- The ONLY source of truth is `./lr2oraja-java` Java source code.

### Mechanical Line-by-Line Translation

Every Java method/class MUST be translated mechanically to Rust. No shortcuts.

| Java | Rust |
|------|------|
| `if (a != null && a.x > 0)` | `if let Some(a) = &a { if a.x > 0 { ... } }` |
| `for (int i=0; i<n; i++)` | `for i in 0..n { ... }` |
| `switch-case` with fallthrough | Replicate exact control flow (no simplification) |
| `int → long` implicit cast | Explicit `as i64` cast |
| `float → double` implicit cast | Explicit `as f64` cast |
| `ArrayList<T>` | `Vec<T>` |
| `HashMap<K,V>` | `HashMap<K,V>` |
| `null` | `Option<T>` |
| `try-catch` | `Result<T>` / `anyhow` |
| `synchronized` | `Mutex` / `RwLock` |
| `static` field | `lazy_static!` / `OnceLock` |

### Six Principles

1. **ZERO improvements allowed** — If Java is verbose, copy it verbatim. Refactoring happens ONLY after ALL tests pass.
2. **Translate one method → test immediately** — Extract expected values from Java, write test, move to next only when green.
3. **Inject debug output into Java** — Export intermediate values as JSON. Compare line by line with Rust output (Golden Master).
4. **Preserve ALL branch/loop structure** — Including `switch-case` fallthrough. NEVER change control flow.
5. **Copy constants and magic numbers AS-IS** — Do NOT rename. Do NOT "improve" names. Make it work first.
6. **Write ALL type conversions explicitly** — Every Java implicit cast must become an explicit Rust cast.

## Workflow (Per Method)

```
1. Read the ENTIRE Java method (never skip lines)
2. Translate line-by-line to Rust (preserve structure exactly)
3. Add JSON debug output to Java side for intermediate values
4. Run both Java and Rust with same input
5. Compare outputs — fix until diff is ZERO
6. Move to next method
```

## Directory Structure

```
brs/
  lr2oraja-java/           # Java source (reference implementation, read-only except debug output)
  lr2oraja-rust/           # Rust port (Cargo workspace)
    crates/                # Rust crates (to be created incrementally)
    golden-master/         # Test infrastructure (Java exporter + fixtures)
    test-bms/              # Test BMS files
  .claude/                 # Claude workspace (plans only)
```

## Tech Stack

| Area        | Java            | Rust                  |
| ----------- | --------------- | --------------------- |
| Graphics    | LibGDX (LWJGL3) | Bevy                  |
| Audio       | PortAudio / GDX | Kira                  |
| Skin (Lua)  | LuaJ            | mlua                  |
| Database    | SQLite (JDBC)   | rusqlite              |
| Timing      | long (μs)       | i64 (μs)              |
| GUI         | JavaFX / ImGui  | egui                  |
| Discord RPC | JNA IPC         | discord-rich-presence |
| OBS         | WebSocket       | tokio-tungstenite     |

## Key Invariants

- All timing uses integer microseconds (i64) — no floating-point drift.
- `java.util.Random(seed)` LCG must be reproduced EXACTLY for pattern shuffle.
- LR2 judge scaling (`lr2JudgeScaling`) uses pure integer arithmetic.
- LongNote references use index-based approach (no circular references).

## Testing Rules

- **Golden Master Testing:** Export Java internal state as JSON, compare against Rust output.
- **TDD:** Red-Green-Refactor for every method.
- **Java Modifications Allowed:** Adding debug output / JSON export methods to Java code is permitted.
- **Test BMS Files:** Create minimal BMS files for each feature under test.
- **Tolerance:** Use ±2μs tolerance for timing comparisons (BPM → μs rounding).

### Golden Master Testing Lessons

- **Java BMSDecoder hardcodes MS932:** UTF-8 BMS metadata may be garbled on Java side.
- **`#RANDOM` is deterministic via `random_seeds.json`:** Use matching `decode_with_randoms(...)` inputs.
- **Avoid JavaFX dependencies:** Keep GM exporter in separate `golden-master` Gradle subproject.
- **Regenerate fixtures with `just golden-master-gen`:** Always regenerate after modifying Java exporter.
- **Parser fixture names keep source extensions:** Use `filename.ext.json` to avoid collisions.

## Implementation Status

- **Phase 1 complete:** `bms-model` (15 modules), `bms-table` (11 modules)
- **Phase 2 complete:** `bmson` (16 model types + BMSONDecoder), `osu` (9 model types + OSUDecoder)
- **Phase 3 complete:** `beatoraja-common` (3 modules), `discord-rpc` (4 modules), `beatoraja-input` (9 modules), `beatoraja-audio` (13 modules), `md-processor` (10 modules)
- **Phase 4 complete:** `beatoraja-core` (47 modules — config types, data models, DB accessors, core/resource types, config subpackage)
- **Phase 5 complete:** `beatoraja-pattern` (14 modules — lane/note shuffle, modifiers), `beatoraja-play` (28 modules — judge, gauge, BGA, game loop)
- **Phase 6 complete:** `beatoraja-skin` (50+ modules — skin rendering engine, property binding, JSON/LR2/Lua skin loaders)

## Deferred / Stub Items

- Phase 7+ type dependencies (screen implementations, select bar, etc.) are stubbed in `beatoraja-skin/src/stubs.rs`
- Phase 4 type dependencies (Config, PlayModeConfig, etc.) are stubbed in each Phase 3 crate's `stubs.rs` (will be replaced with imports from `beatoraja-core`)
- PortAudio, LibGDX, ebur128, 7z extraction methods use `todo!()` pending external library integration
- javax.sound.midi equivalents stubbed (no direct Rust equivalent)
- MIDI device enumeration stubbed
- FLAC/MP3 decoding deferred to library integration
- Skin rendering (PlaySkin, SkinNote, SkinGauge, etc.) have stub implementations pending Phase 6 Skin system
- BGA video processing (FFmpegProcessor, GdxVideoProcessor) uses `todo!()` pending video library integration
- MainController-dependent methods in TargetProperty return stub data

## Translation Lessons Learned

> **This section is a living document.** Update it after every phase with new patterns, pitfalls, and decisions discovered during translation.

### Java Class Hierarchy → Rust Enum

Java abstract classes with `instanceof` checks translate best as Rust enums with a shared data struct:

```
Java:  abstract class Note { fields... }
       class NormalNote extends Note
       class LongNote extends Note { pair, end, type }
       class MineNote extends Note { damage }

Rust:  struct NoteData { /* shared fields */ }
       enum Note { Normal(NoteData), Long { data: NoteData, ... }, Mine { data: NoteData, ... } }
```

This preserves the `instanceof` pattern as `match` / `if let` and avoids trait-object complexity.

### f64 as BTreeMap Key

`f64` does not implement `Ord` in Rust. When Java uses `TreeMap<Double, V>`, convert the key to `u64` via `f64::to_bits()` for the `BTreeMap`, or use a newtype wrapper with manual `Ord` impl. Phase 1 used `to_bits()`.

### Borrow Checker vs. Java Constructor Patterns

Java constructors that take `this` and a sibling object (e.g., `Section(model, prev, ...)`) cause borrow issues when both `&self` and `&mut model` are needed. Solution: pass extracted primitive values (`prev_sectionnum`, `prev_rate`) instead of `Option<&Section>`.

### MS932 Encoding

Java's `BMSDecoder` hardcodes `"MS932"` (Shift_JIS superset). Use `encoding_rs::SHIFT_JIS` in Rust:

```rust
let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(raw_bytes);
```

### Parallel Agent Strategy

Independent crates (no cross-dependencies) should be translated by parallel agents writing to separate directories. This was done successfully with `bms-model` and `bms-table` in Phase 1.

**Pitfall:** Each agent must commit ALL files it creates, including `Cargo.toml`. Verify with `git status` after agents complete. In Phase 1, `bms-model/Cargo.toml` was missed and required a follow-up commit.

### Workspace Cargo.toml Must Exist Before Agents Start

Create the workspace `Cargo.toml` and all crate `Cargo.toml` files **before** launching translation agents. Agents need `cargo check` to work, which requires the workspace to be configured.

### Stub lib.rs for Sibling Crates

When translating crate A, the workspace may fail to compile if crate B (referenced as workspace member) has no `lib.rs`. Create a stub `lib.rs` (empty or with just module declarations) for all workspace members before starting translation.

### CommandWord Enum Translation

Java enums with `BiFunction` fields (like `CommandWord` in `BMSDecoder.java`) translate to a `match` on enum variants calling closures, or a function-dispatch table. Phase 1 used a match-based approach.

### Java TreeMap Iteration Order

`TreeMap` in Java iterates in key order. `BTreeMap` in Rust provides the same guarantee. Always use `BTreeMap` (not `HashMap`) when Java uses `TreeMap`, especially for section/timeline processing where order matters.

### BMSON JSON Model Types (Phase 2)

Java BMSON model classes with `@JsonIgnoreProperties(ignoreUnknown=true)` translate to Rust structs with `#[serde(default)]` and `#[derive(Deserialize)]`. Fields that can be `null` in JSON (checked with `!= null` in Java) should use `Option<T>`. Use `#[serde(default)]` on the struct to handle missing fields.

### Java switch-case Fallthrough (Phase 2)

The Java `Osu.java` parser has a `switch(section)` where the `"General"` case falls through to `"Editor"` (missing `break`). In Rust, `match` does not fall through. Replicate the fallthrough by explicitly calling the Editor parser at the end of the General branch.

### BTreeMap `lowerEntry` and `subMap` (Phase 2)

- Java `TreeMap.lowerEntry(y)` → Rust `BTreeMap::range(..y).next_back()`
- Java `TreeMap.subMap(y1, false, y2, true)` → Rust `BTreeMap::range((Excluded(y1), Included(y2)))`

### LongNote Pairing Without Direct References (Phase 2)

Java's BMSONDecoder uses direct object references for LN pairs (`ln.setPair(lnend)`). In Rust, use section-based tracking (`BmsonLnInfo { start_section, end_section, end_y }`) and timeline key lookups to modify pair notes. The `end_y` field allows locating the end note's timeline for wav/starttime/duration assignment.

### Submodule Organization (Phase 2)

When Java packages (like `bms.model.bmson` with 16 small classes) translate to Rust, consolidate all types into a single `mod.rs` file rather than one file per type. This reduces file count and simplifies imports.

### Platform-Specific Code with cfg (Phase 3)

Java platform detection (`System.getProperty("os.name")`) translates to `#[cfg(unix)]` / `#[cfg(windows)]` conditional compilation. Discord RPC's IPC uses Unix domain sockets on Linux/macOS and Named Pipes on Windows — keep both implementations with platform gates.

### Stub Modules for Phase 4 Dependencies (Phase 3)

Phase 3 crates reference Phase 4 types (Config, PlayModeConfig, Resolution, etc.) that don't exist yet. Create a `stubs.rs` module in each crate with minimal struct/trait definitions. These stubs will be replaced when Phase 4 is translated.

### Java Back-References → Callback Traits (Phase 3)

Java patterns where child objects hold `this` references to parent (e.g., `BMSPlayerInputProcessor` holding `BMSPlayer`) cause borrow conflicts in Rust. Solution: define a callback trait (e.g., `BMSPlayerInputDevice`) and pass `&mut dyn Trait` to methods instead of holding permanent references.

### PCM Arc Sharing for Slice (Phase 3)

Java's `ShortDirectPCM` uses `ByteBuffer.slice()` for zero-copy views. In Rust, use `Arc<Vec<T>>` with offset/length fields. The `slice()` method creates a new struct sharing the same `Arc` data with adjusted offset — avoids copying sample data.

### MS-ADPCM Decoder (Phase 3)

Java's MS-ADPCM decoder uses mutable coefficient arrays and predictor state. Translate as a stateless function taking `&[u8]` input and returning `Vec<i16>`. The adaptation table and coefficient sets are static constants.

### Java POJO Config Classes → Rust pub fields + serde (Phase 4)

Java config classes (Config, PlayerConfig, PlayModeConfig, AudioConfig, etc.) with private fields + getter/setter pairs translate best as Rust structs with `pub` fields and `#[derive(Clone, Debug, Default, Serialize, Deserialize)]`. Use `#[serde(default)]` on the struct level so missing JSON fields get default values. This avoids verbose getter/setter boilerplate while preserving JSON serialization compatibility.

### Java JDBC → rusqlite (Phase 4)

Java `PreparedStatement` + `ResultSet` patterns translate to rusqlite's `prepare` + `query_map`/`query_row`. Use `params![]` macro for bind parameters. The SQL strings can be copied nearly verbatim — rusqlite uses `?` placeholders just like JDBC. For database accessor inheritance (e.g., `ScoreDatabaseAccessor extends SQLiteDatabaseAccessor`), use composition (embed the base struct) instead of trait inheritance.

### Large Hub Crate with Phase 5+ Stubs (Phase 4)

Phase 4 is the "hub" module — most later phases depend on it, and it references many Phase 5+ types. Create a comprehensive `stubs.rs` with minimal implementations of external types (JudgeAlgorithm, BMSPlayerRule, SkinType, GrooveGauge, BarSorter, pattern modifiers, etc.). Methods that depend heavily on unimplemented subsystems should use `todo!("Phase N dependency")`.

### Parallel Agent Translation for Large Phases (Phase 4)

For phases with 40+ files, use a team of 4 agents translating in parallel. Group files by dependency:
- Group A: Foundational config types (no internal deps)
- Group B: Data models (depend on config types)
- Group C: Database accessors (depend on data models)
- Group D: Core/resource types (depend on everything)

Each agent writes to separate files, so no merge conflicts. Pre-create lib.rs with all module declarations and stubs.rs before launching agents.

### LibGDX JSON → serde_json (Phase 4)

Java's LibGDX `Json` class with `setIgnoreUnknownFields(true)` translates to `serde_json::from_str` with `#[serde(default)]` on structs. LibGDX's `Json.prettyPrint()` translates to `serde_json::to_string_pretty()`. The `setUsePrototypes(false)` flag (which disables skipping default values) has no direct equivalent — serde always serializes all fields.

### Java Abstract Class with Factory → Rust Trait + Enum Dispatch (Phase 5)

Java's `PatternModifier` abstract class with a `create()` factory method translates to a Rust trait plus a `PatternModifierBase` struct for shared state (assist, seed, player). Each concrete modifier (LaneMirrorShuffleModifier, etc.) implements the trait. The `create()` factory returns a boxed trait object `Box<dyn PatternModifier>`.

### Java Interface Hierarchy with Abstract Methods → Rust Enum + Match (Phase 5)

Java's `GrooveGauge.GaugeModifier` interface with static lambda fields (TOTAL, LIMIT_INCREMENT, MODIFY_DAMAGE) translates to a Rust enum with a `modify()` method using `match`. This avoids trait objects for simple function-like dispatch.

### LR2 Mersenne Twister (Phase 5)

Java's `LR2Random` class implements a custom Mersenne Twister (MT19937) with LR2-specific seeding. This must be translated exactly — bit operations, unsigned arithmetic, and the tempering step must all match. Use `u32` for internal state with wrapping arithmetic to match Java's int overflow behavior.

### Two-Crate Split for Pattern & Play (Phase 5)

`beatoraja.pattern` and `beatoraja.play` translate to two separate Rust crates: `beatoraja-pattern` depends only on `bms-model` and `beatoraja-core`, while `beatoraja-play` depends on both plus `beatoraja-pattern`, `beatoraja-audio`, and `beatoraja-input`. This split reflects the dependency graph — pattern modifiers are used by the play system but not vice versa.

### Rendering Stubs for Skin Elements (Phase 5)

Skin-related types in the play package (SkinNote, SkinGauge, SkinJudge, SkinHidden, SkinBGA, PlaySkin) depend heavily on LibGDX rendering primitives (Texture, SpriteBatch, TextureRegion). Stub these in `stubs.rs` and mark rendering methods with `todo!("Phase 6+ dependency")`. The data structures and logic can still be translated mechanically.

### Java Random vs Rust rand (Phase 5)

Java's `java.util.Random(seed)` uses a specific LCG algorithm. For pattern shuffle reproducibility, use `rand::rngs::StdRng::seed_from_u64(seed)` or `SmallRng` — but note that exact random sequences will differ. If exact Java Random reproduction is needed, implement the Java LCG manually (multiplier=0x5DEECE66D, addend=0xB, mask=(1L<<48)-1).

### LibGDX Rendering Types as Stubs (Phase 6)

The skin system depends heavily on LibGDX types (TextureRegion, Texture, SpriteBatch, BitmapFont, ShaderProgram, Pixmap, Matrix4, etc.). Create comprehensive stubs in `stubs.rs` with `#[derive(Clone, Default, Debug, PartialEq)]` so they can be used in collections and comparisons. Actual rendering integration is deferred to a future graphics integration phase.

### Java Interface → Rust Trait with Box<dyn> (Phase 6)

Java property interfaces (BooleanProperty, FloatProperty, IntegerProperty, StringProperty, TimerProperty, Event) translate to Rust traits with `Box<dyn Trait>` for polymorphism. Factory classes (e.g., `BooleanPropertyFactory.getBooleanProperty(id)`) become functions returning `Option<Box<dyn Trait>>` with a large `match` on the integer ID.

### Vec<Option<T>> for Nullable Java Arrays (Phase 6)

Java `TextureRegion[]` arrays where elements can be `null` translate to `Vec<Option<TextureRegion>>`. However, many callers construct these from non-null Vecs. Solution: provide convenience constructors like `new_with_int_timer_from_vec(images: Vec<TextureRegion>, ...)` that wraps each element in `Some()`, alongside the canonical `new_with_int_timer(images: Vec<Option<TextureRegion>>, ...)`.

### Five-Agent Split for Large Skin Phase (Phase 6)

Phase 6 has 73 Java files (~19K lines). Split into 5 parallel agent groups:
- Property traits & factories (13 files)
- Base types (skin_type, skin_property, sources — 15 files)
- Rendering objects (skin_object, skin_image, skin_graph, etc. — 19 files)
- JSON skin loaders (11 files)
- LR2 + Lua skin loaders (15 files)

Pre-create all module stubs and lib.rs before launching agents. Monitor agent completion — if any agent shuts down mid-work, identify remaining stub files (1-line files) and launch replacement agents.

### SkinObject Mega-Class Pattern (Phase 6)

Java's `SkinObject` is a ~1200-line class with extensive rendering logic (draw_image, color/rotation/stretch transformations, region calculations). Translate as a single `SkinObjectData` struct with methods. The `SkinObjectRenderer` struct wraps SpriteBatch state. Use `pub` fields for cross-struct access patterns (e.g., `skin_image.data.draw`).

### Factory Function Naming Convention (Phase 6)

Java factory classes use method names like `getIntegerProperty(id)`. In Rust, append `_by_id` suffix to distinguish from other overloads: `get_integer_property_by_id(id)`, `get_rate_property_by_id(id)`, `get_image_index_property_by_id(id)`. All callers must use the exact function name — agents sometimes use the shorter form, causing compilation errors across multiple files.
