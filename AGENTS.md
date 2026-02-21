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

Fresh start. No Rust code exists yet.

## Deferred / Stub Items

(None — clean slate)
