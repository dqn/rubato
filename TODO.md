# Porting TODO — Remaining Work

Phases 1–44 complete. **2391 tests, 16 ignored.** 27 crates, 127k lines. See AGENTS.md.

---

## Phase 45: Regression Fixes (no blockers)

- [x] **45a:** RandomizerBase StdRng → JavaRandom — Replace `StdRng` with `JavaRandom` in `beatoraja-pattern/src/randomizer.rs` (field, new(), set_random_seed(), all closure signatures). Also fix `MineNoteModifier` and `LongNoteModifier` using `rand::random()` instead of seeded RNG
  - depends: none
- [x] **45b:** Serde field name mismatches in ScoreData — Add `#[serde(rename)]` for `total_duration`→`totalDuration`, `total_avg`→`totalAvg`, `device_type`→`deviceType`, `judge_algorithm`→`judgeAlgorithm`, and rename `combo`→`maxcombo` in `beatoraja-types/src/score_data.rs` + all callers
  - depends: none

## Phase 46: Core Lifecycle Wiring (unblocks gameplay)

- [x] **46a:** PlayerResource.loadBMSModel() — Wire `ChartDecoderImpl::get_decoder()` into `player_resource.rs:set_bms_file()`. Change `model` field from `Option<()>` to real `BMSModel`. Call `BMSModelUtils::set_start_note_time()` and `BMSPlayerRule::validate()`
  - depends: none
- [x] **46b:** PlayerResource.SongData type unification — Replace local `SongData` stub in `beatoraja-core/src/player_resource.rs` with `beatoraja_types::song_data::SongData`. Construct via `SongData::new_from_bms_model()` in `set_bms_file()`
  - depends: 46a
- [x] **46c:** MainController.exit() and save_config() — Implement real exit logic and config serialization in `beatoraja-core/src/main_controller.rs`
  - depends: none

## Phase 47: Skin Rendering Pipeline (makes screens visible)

- [x] **47a:** FloatPropertyFactory implementation — Replace stub `get() → 0.0` with real delegate calls to MainState. Decide architecture: trait method extension vs `dyn Any` downcast for ~50 property entries in `beatoraja-skin/src/property/float_property_factory.rs`
  - depends: none
- [x] **47b:** Timer stub replacement — Replace zero-return timer with real timer manager access in `beatoraja-skin`
  - depends: none
- [x] **47c:** MainState.load_skin() per-state overrides — Add `load_skin()` override to CourseResult, MusicResult, PlayState, DecideState following MusicSelector's pattern
  - depends: none
- [x] **47d:** SkinFloat enum variant — Add SkinFloat to SkinObject enum + dispatch in beatoraja-skin
  - depends: none
- [x] **47e:** BooleanPropertyFactory stubs — Implement remaining boolean property delegates
  - depends: none

## Phase 48: Select→Play Wiring

- [x] **48a:** Bar Clone problem resolution — Resolve `Bar` enum Clone issue (TableAccessor `dyn` → concrete enum or `Arc` shared ownership) in `beatoraja-select/src/bar/bar.rs`
  - depends: none
- [x] **48b:** Bar get_children() stubs (7 types) — Implement `get_children()` for FolderBar, HashBar, SearchWordBar, SameFolderBar, CommandBar, LeaderBoardBar, DirectoryBar by threading `SongDatabaseAccessor`
  - depends: 48a
- [x] **48c:** read_chart/read_course/read_random_course — Wire select→play state transitions in `beatoraja-select/src/music_selector.rs` via PlayerResource
  - depends: 46a, 46b, 48b

## Phase 49: Play State Integration

- [x] **49a:** bms_player.rs Phase 22 wiring — Resolve 19 TODO items for input/transition/config wiring in `beatoraja-play/src/bms_player.rs`
  - depends: 46a, 46b
- [x] **49b:** LaneRenderer.draw_lane() — Port 713-line Java rendering method in `beatoraja-play/src/lane_renderer.rs`
  - depends: 47a, 47c

## Phase 50: Result & Course Integration

- [x] **50a:** CourseResult MainState wiring — Wire create/prepare/render/input to MainController and PlayerResource in `beatoraja-result/src/course_result.rs`
  - depends: 46a, 47c
- [x] **50b:** CourseResult IR thread — Spawn IR send thread in CourseResult prepare()
  - depends: 50a

## Phase 51: Skin Loaders Completion

- [x] **51a:** Lua MainStateAccessor — Implement 20 missing functions in beatoraja-skin Lua bridge
  - depends: 47a
- [x] **51b:** LR2 21 commands — Implement 21 stubbed LR2 skin commands
  - depends: 47a
- [x] **51c:** JSON 7 skin factories — Implement 7 stubbed JSON skin factories
  - depends: 47a
- [x] **51d:** SkinTextFont.draw_with_offset() — Implement TrueType text rendering with alignment, shadow, overflow modes
  - depends: none

## Phase 52: Launcher & External Wiring

- [x] **52a:** Skin header loading wiring — Connect skin header loader in beatoraja-launcher
  - depends: 47c
- [x] **52b:** Async BMS DB loading — Implement async song database loading in launcher
  - depends: none
- [x] **52c:** get_screen_type() implementation — Replace `→ Other` stubs in 3 external files
  - depends: none
- [x] **52d:** DifficultyTableParser bridge — Wire bms-table crate as dependency + toSongData()
  - depends: none

## Phase 53: Quality & Test Coverage

- [x] **53a:** beatoraja-modmenu tests — Add 58 tests covering imgui_notify, download_task, freq_trainer, skin_menu, random_trainer, misc_setting
  - depends: none
- [x] **53b:** beatoraja-ir tests — Add 40+ tests covering ir_response, ir_score_data, leaderboard_entry, lr2_ghost_data, lr2_ir_connection, ranking_data
  - depends: none
- [x] **53c:** beatoraja-controller tests — Add 28 tests covering lwjgl3_controller button/axis mapping, manager operations
  - depends: none
- [x] **53d:** Remove dead code: beatoraja-common — Removed 785 lines (0 callers), deleted crate from workspace
  - depends: none

---

## 軽微な未移植項目

| 項目 | 影響 | 備考 |
|------|------|------|
| `BMSModel.compareTo()` | 低 | 必要時に Ord 実装可。Java でも未使用 |
| `BMSModelUtils.getAverageNotesPerTime()` | 低 | Java でも未使用 (デッドコード) |
| OBS reconnect lifecycle | 低 | server_uri/password の inner 保持が必要 |
| Skill rating calculation | 低 | Java ソースに実装なし (移植元不在) |

## Permanent Stubs

- **Twitter4j** (`beatoraja-external`): ~446 lines, `bail!()` — API 廃止済みのため意図的に未実装
- **ShortDirectPCM** (`beatoraja-audio`): Java 固有の DirectBuffer — Rust では不要
