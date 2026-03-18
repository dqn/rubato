# lr2oraja Rust Porting

beatoraja fork (Java 313 files / 72k+ lines) → Rust. 25 crates, 167k lines.

## Rules

### Porting

- Golden Master: pre-generated JSON fixtures in `golden-master/fixtures/`. Tolerance: +-2us.
- Preserve ALL branch/loop/fallthrough structure. Copy constants/magic numbers AS-IS.
- Explicit type conversions -- every implicit Java cast → explicit Rust cast.
- Treat "mechanical translation complete", "zero stubs", and "runtime parity verified" as separate milestones. Do not declare a phase complete based on compile success, test counts, or stub counts alone.
- During Java parity review, explicitly audit RNG, serde field names and aliases, tolerant JSON/Gson coercions, timezone and DST behavior, truncating numeric casts, and path/CRC semantics.
- When choosing between similar-sounding APIs (e.g., `has_long_note()` vs `has_undefined_long_note()`, `maxbpm` vs `mainbpm`), trace back to the Java source to verify which semantic is needed.
- NEVER use blocking I/O (`rx.recv()`, synchronous HTTP) on the main/render thread. Use background threads + `try_recv()` or non-blocking poll.

### Operations

- After completing a phase/task, update TODO.md and AGENTS.md.
- Worktree isolation: **always merge worktree branches before sending shutdown requests**.
- Worktree cleanup: **always run `rm -rf target/` inside a worktree before removing it** to avoid multi-GB disk waste (4+ GB each).
- After broad renames, file splits, crate consolidation, or other structural refactors, run runtime smoke on every affected entrypoint immediately. Build/lint green is insufficient.

### Debugging

- For black screens, no-op inputs, broken transitions, or silent state desync, investigate wiring and lifecycle boundaries before rewriting business logic. Checklist: input, timer, audio, `skin.prepare()`, skin property delegation, state transitions, controller/resource sync, interactive mouse context.
- When writing or reviewing `create()`, `init()`, `load()`, or `prepare()` in state objects, enumerate ALL subsystems (input, audio, gauge, replay, BGA, callbacks, timers, skin) and verify each is initialized. "It compiles" does not mean "it's wired."
- When passing data between states via handoff structs (`ScoreHandoff`, `StateCreateEffects`), verify every field is populated at the source and consumed at the destination. Empty/default fields in handoff structs are bugs.

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
rubato/              # Cargo workspace (15 crates) at repo root
  crates/
    bms-model        # BMS/BME/BML parser + model
    bms-table        # Difficulty table parser
    rubato-types     # Shared types (circular dep breaker)
    rubato-audio     # Audio (Kira 0.12)
    rubato-input     # Keyboard/controller input (+ controller)
    rubato-render    # Rendering (wgpu)
    rubato-skin      # Skin loading/layout
    rubato-song      # Song DB (rusqlite, + md-processor)
    rubato-core      # State machine, main loop (+ pattern)
    rubato-play      # Play state (gameplay)
    rubato-state     # Select/Decide/Result/Modmenu/Stream states
    rubato-ir        # Internet ranking
    rubato-external  # Twitter, clipboard, Discord RPC, OBS WebSocket
    rubato-launcher  # Launcher UI (egui)
    rubato-bin       # Entry point
  golden-master/   # Golden Master test infra
  test-bms/        # Test BMS files
  .claude/tmp/beatoraja/  # Original Java source code (beatoraja fork)
```

## Key Invariants

- Timing: i64 microseconds. JavaRandom LCG in `rubato-core::pattern` (**never** `StdRng`/`rand`). LR2 MT19937. LR2 judge: pure integer arithmetic. LongNote: index-based.

## Testing

- **Test runner:** `just test` (excludes slow render snapshot tests and `rubato-bin` which requires ffmpeg system library) or `just test-all` (full, requires ffmpeg).
- **Golden Master:** `just golden-master-test`. Fixtures: `golden-master/fixtures/*.json` (pre-generated).
- **TDD:** Red-Green-Refactor.

## Status

**5757 tests.** All 62 phases complete. Zero clippy warnings. Zero regressions.

- **Migration:** 4,279 Java methods resolved (4,049 direct + 230 architectural redesigns). 0 functional gaps. ast-compare retired.
- **Stubs:** 0 remaining. All 151 resolved (Phase 58-62). All 32 debug stubs resolved (12 implemented, 20 compile-time comments).
- **Permanent stubs (intentional):**
  - Twitter4j (`rubato-external`): ~446 lines, `bail!()` -- API deprecated
  - ShortDirectPCM (`rubato-audio`): Java-specific DirectBuffer -- unnecessary in Rust
  - JavaFX find_parent_by_class_simple_name (`rubato-launcher`): No egui equivalent
  - randomtrainer.dat (`rubato-modmenu`): Binary resource from Java, uses empty HashMap fallback

## Lessons Learned

### Porting Mechanics

- **Panic surface audit (24% of all fix commits):** Java has implicit safety (null returns, silent overflow, bounds defaults) that becomes panics in Rust. Systematically audit every: array/slice index, division, `as` cast (especially to unsigned), `.unwrap()`, and enum match.
- **Explicit Drop required:** Java's GC handles cleanup implicitly. Every ported background thread, network connection, audio handle, and GPU resource needs `Drop` impl or `dispose()`. Add cleanup paths immediately, not in a later audit.
- **Borrow checker patterns:** `&mut` conflicts → scoped block. Self-ref → `Option::take()`. Parent ref → callback trait. `Box<dyn Trait>` blocks Clone → use `Arc<dyn Trait>`.
- **Stubs management:** `stubs.rs` per crate → replace via `pub use`. Always `cargo check` after removal.
- **Circular deps:** `rubato-types` for shared types. Core cannot import: song, skin, play, select, result, ir, modmenu.
- **Java float→int→byte truncation:** Use `as i32 as i8` in Rust (via i32 for truncation). Direct `as i8` saturates since Rust 1.45.

### Encoding & Serialization

- **SHIFT_JIS everywhere:** All file readers for Japanese formats (.chp, .lr2skin CSV, .bms) must use `encoding_rs::SHIFT_JIS`, not UTF-8. Use `std::fs::read()` + `SHIFT_JIS.decode()`.
- **CRC32 path encoding:** Java's `String.getBytes()` uses MS932 on Windows. CRC32 over file paths must encode as Shift_JIS bytes, not UTF-8, or Japanese paths produce different hashes.
- **Serde alias vs rename:** `#[serde(alias)]` is deserialization-only. For bidirectional Java parity, use `#[serde(rename)]`. Convention: `BPM`→`Bpm`, `URL`→`Url`.
- **Lua→JSON coercion:** 3-layer: numbers→strings, float→int truncation, empty `{}`→remove.
- **String byte slicing safety:** Never slice at byte positions from external data without `is_char_boundary()` / `floor_char_boundary()` / `strip_prefix()`.

### State Lifecycle & Wiring

- **Wiring-first debugging:** Black screens, no-op interactions, broken transitions are almost always wiring/lifecycle faults, not algorithm mistakes.
- **Controller wiring across crates:** States that cannot own `&mut MainController` use a queued `MainControllerAccess` proxy + MainController-side drain step. `Arc<Mutex<State>>` wrappers must sync `MainStateData` bidirectionally.
- **Skin event wiring:** `SkinDrawable` mouse/custom-event paths need a state-aware `SkinRenderContext`, not a timer-only adapter. Interactive states build their own context for mouse/render passes.
- **Mouse bridge parity:** States with clickable skins must override `handle_skin_mouse_pressed()` / `handle_skin_mouse_dragged()`, take ownership of `skin` + `TimerManager`, and build state-aware context.
- **Lua skin load-time context:** Lua skins read `main_state.*` during `load_header()` / `load()`, not only render. Pass a state-aware adapter implementing `rubato_skin::MainState` into the loader.
- **State create() must init all subsystems:** Every subsystem referenced in render/input paths (gauge, replay, BGA, timers, etc.) must be initialized in `create()`. Omitting gauge init causes silent failures: 0% rendering, skipped stage-failed, empty gauge log.
- **Audio side effects:** States needing the real `AudioDriver` after render/shutdown should flush through `MainState::sync_audio()` from `MainController`, not through queued commands.
- **Play config remap must clear live key state:** `set_play_config()` must clear live key/button/MIDI state before installing new mappings, or play starts with stuck beams and false autoplay.
- **Ranking cache sharing:** IR ranking cache handles must stay shared across `MainController`, proxies, and result wrappers. Fresh per-wrapper caches break select→play/result reuse.
- **Frozen harness timer sync:** `MainController.timer` and active state's `main_state_data.timer` are separate clocks. Test harnesses must advance both in lockstep.
- **Mouse context cannot dispatch custom events:** All mouse contexts (`DecideMouseContext`, `ResultMouseContext`, etc.) share the same borrow limitation: the skin is `take()`-ed before the mouse context is created, so `execute_event` cannot call back to `skin.execute_custom_event()`. Custom events (1000-1999) from `DelegateEvent` clickevents are silently dropped during mouse handling. Non-custom events (state changes, timer sets, config cycles) work because they use direct `Event` implementations that call `change_state`/`set_timer_micro`/`player_config_mut` directly, bypassing `execute_event`. Fixing this requires either queued event dispatch or restructuring the `take()`-based borrow pattern.

### Rendering & Skin Pipeline

- **Audit the full chain in one pass:** When fixing a rendering subsystem, audit asset load → font/texture resolve → coordinate scaling → positioning → draw call together. Single-layer fixes cause cascading commit chains.
- **Path resolution:** Skin/config asset paths resolve against both configured skin root AND ancestor directories of CWD. Font paths must be resolved relative to the current skin file before object conversion. Fallback files like `config_player.json` resolve relative to the player/config root and current-directory ancestors.
- **Lua/JSON source resolution:** Preserve filemap substitutions and wildcard expansion through runtime texture resolution. Do not reject image objects with `*` in source path early; resolve filemap first.
- **Bitmap font parity:** Scale both destination rectangle AND text `size` by the destination-width ratio (matching Java's `JsonSkinObjectLoader`). Glyph `yoffset` uses BMFont top-origin without reapplying font base offset.
- **Render-capture testing:** Runtime glyph quads use generated keys (`__pixmap_*`), not source `.fnt` paths. Filter on actual emitted identifiers + target widget bounds. Treat hardcoded spatial expectations as hypotheses; observe actual distribution first.
- **Select bar subobject scaling:** `SelectBarData` children bypass `Skin::set_destination()`. Apply `src`→`dstr` scaling when extracting songlist bar subobjects manually.
- **Skin loader safe division:** LR2 CSV src.width/src.height can be zero. All `dst / src` divisions must use `safe_div_f32()` from rubato-skin.
- **Skin property ref parity:** LR2 numeric refs and image-index refs reuse the same ID for different meanings. Keep `integer_value(id)` and `image_index_value(id)` separate.
- **LR2 option handoff:** Copy `#SETOPTION` / custom option selections from loader into `Skin` before `prepare()`, which removes option-gated objects based on `Skin.option`.
- **Property delegate pattern:** `integer_value(id)` / `float_value(id)` / `boolean_value(id)` on MainState; skin property factories delegate via ID lookup.
- **Active skin verification:** Green tests on default JSON skin do not verify the user's configured Lua/bitmap-font skin. Check `config_player.json` and active profile first.

### Input & Gameplay

- **Play input handoff:** Rust state hooks must explicitly copy START/SELECT/key/control/scroll/device state from `BMSPlayerInputProcessor`, and write back consumed flags after processing.
- **Analog input handoff:** Snapshot `is_analog`, `get_analog_diff()`, `get_time_since_last_analog_reset()` during `sync_input_from()`, flush with `reset_analog_input()` in `sync_input_back_to()`.
- **Key beam release parity:** Only the pressed branch is gated by `isJudgeStarted` / autoplay. Release must always flip KEYON → KEYOFF when the timer is on.
- **Judge timer parity:** Queue and apply judge/combo timer side effects (`46/47/247`, `446/447/448`, bomb timers) on the main thread when a judgment lands.
- **Play note render timing:** `LaneRenderer::draw_lane()` expects timer start timestamps, not elapsed durations. Pass `timer(TIMER_PLAY)` and let the renderer subtract from `now_time()`.
- **Lane renderer coordinate parity:** SpriteBatch uses Y-up projection. Keep Java's `hu`/`hl`/upward break condition/positive LN span semantics. Do not rewrite as Y-down.
- **Selected play-config lookup:** Resolve play config from the selected bar's actual mode, not only the current selector mode.
- **Target list fallback:** When `TargetProperty`'s global list is empty, fall back to `player_config.targetlist`.

### Render Context Adapter Completeness

- **Trait adapter delegation is the #1 blind spot in IS-A→composition ports (24% of RFL round 1 fixes).** Java `BMSPlayer extends MainState` means all methods are automatically inherited. Rust adapter structs (`PlayRenderContext`, `ResultRenderContext`, etc.) must explicitly delegate every `SkinRenderContext` method that skin objects call. Missing delegations silently return trait defaults (None, false, 0, empty Vec) instead of erroring. When adding a new render context or modifying the `SkinRenderContext` trait, enumerate ALL callers (skin objects, Lua accessors, property factories) and verify the adapter delegates each one.
- **Course-mode-only data flows escape single-song testing.** Course gauge constraints, previous-stage gauge restoration, course aggregate score checks for clear/fail, and course gauge history are all gated by `is_course_mode()` conditions. Always test course-specific paths explicitly; single-song green tests prove nothing about course behavior.
- **UTF-8 byte processing: never use `byte as char`.** Converting a raw byte to `char` via `u8 as char` expands it to its Latin-1 code point. For multi-byte UTF-8 (Japanese text), this silently corrupts every non-ASCII character. Use `Vec<u8>` output buffers with `String::from_utf8()` at the end.

### Testing & Verification

- **Process-global test state:** Use RAII guard patterns (guard struct with `Drop`) for tests modifying static Mutex / singletons / env vars. Without guards, parallel nextest causes non-deterministic interference.
- **GUI subprocess smoke tests:** Use explicit timeouts, not `Command::output()` alone. Healthy GUI event loops run indefinitely, turning smoke tests into hangs.

## Issue Tracking with bd (beads)

This project uses **bd (beads)** for ALL issue tracking. Do NOT use markdown TODOs, task lists, or other tracking methods.

### Quick Start

```bash
bd ready --json          # Check for unblocked work
bd create "Title" --description="Context" -t bug|feature|task -p 0-4 --json
bd update <id> --claim --json
bd close <id> --reason "Completed" --json
```

### Issue Types

- `bug` - Something broken
- `feature` - New functionality
- `task` - Work item (tests, docs, refactoring)
- `epic` - Large feature with subtasks
- `chore` - Maintenance (dependencies, tooling)

### Priorities

- `0` - Critical (security, data loss, broken builds)
- `1` - High (major features, important bugs)
- `2` - Medium (default, nice-to-have)
- `3` - Low (polish, optimization)
- `4` - Backlog (future ideas)

### Workflow for AI Agents

1. **Check ready work**: `bd ready` shows unblocked issues
2. **Claim your task**: `bd update <id> --claim`
3. **Work on it**: Implement, test, document
4. **Discover new work?** Create linked issue:
   - `bd create "Found bug" --description="Details" -p 1 --deps discovered-from:<parent-id>`
5. **Complete**: `bd close <id> --reason "Done"`

### Sync

- Each write auto-commits to Dolt history
- Use `bd dolt push` / `bd dolt pull` for remote sync

## Landing the Plane (Session Completion)

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **Sync beads** - `bd dolt push`
5. **Clean up** - Clear stashes, prune remote branches
6. **Hand off** - Provide context for next session
