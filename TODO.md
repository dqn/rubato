# Porting TODO — Remaining Work

Phases 1–57 complete. **2940 tests.** 26 crates, 158k lines. See AGENTS.md.

---

## Completed Phases (45–57)

<details>
<summary>Phase 45–53: Regression fixes, lifecycle wiring, skin rendering, select→play, result, skin loaders, launcher, quality</summary>

- **45a–b:** RandomizerBase JavaRandom, ScoreData serde
- **46a–c:** PlayerResource.loadBMSModel, SongData unification, MainController.exit/save_config
- **47a–e:** FloatPropertyFactory, Timer, load_skin overrides, SkinFloat, BooleanPropertyFactory
- **48a–c:** Bar Clone, get_children (7 types), read_chart/read_course
- **49a–b:** bms_player wiring, LaneRenderer.draw_lane
- **50a–b:** CourseResult MainState, CourseResult IR thread
- **51a–d:** Lua MainStateAccessor, LR2 21 commands, JSON 7 factories, SkinTextFont
- **52a–d:** Skin header loading, async BMS DB, get_screen_type, DifficultyTableParser
- **53a–d:** modmenu/ir/controller tests, dead code removal (beatoraja-common)

</details>

<details>
<summary>Phase 54–57: ast-compare audit, BytePCM fix, method-level ignore, final gap resolution</summary>

- **54a:** ast-compare ignore list — bmson/osu POJOs added
- **54b:** BytePCM float→byte — `as i32 as i8` matches Java truncation
- **55:** 28 genuine gaps audited → 15 false positives, 7 implemented, 6 blocked
- **56:** Method-level ignore added to ast-compare. 170 false positives (136 patterns). Gap: 90
- **56b:** 52 additional false positives + PlayerResource.reloadBMSFile. Gap: 38
- **57:** 13 KeyConfiguration + 13 SkinConfiguration methods, 12 false positives. Gap: 0

</details>

---

## Phase 58: "Not Implemented" Message Cleanup (non-functional)

151 箇所の `not yet implemented` / `not implemented` スタブを分類・整理する。

- [ ] **58a:** Test infrastructure stubs (~24) — MockIRConnection `"not implemented"` → `"mock"`, ast-compare `warn!` → `trace!`
- [ ] **58b:** NullSongDatabaseAccessor (6) — `warn!` → `trace!`, doc comment 追加
- [ ] **58c:** Permanently out-of-scope (~6) — javafx_utils, random_trainer, sprite_batch_helper, bms_decoder, key_input_log, skin_source_movie
- [ ] **58d:** Blocked stubs documentation (~18) — メッセージを `"stub: <method> — blocked by <reason>"` 形式に統一 + beads issue 作成

## Phase 59: Sound System Wiring + MusicSelector Navigation (~25)

- [ ] **59a:** MainControllerAccess にサウンドメソッド追加 (play_sound, stop_sound, has_sound)
- [ ] **59b:** MusicResult sound wiring (4)
- [ ] **59c:** CourseResult sound wiring (2)
- [ ] **59d:** MusicSelector state transition wiring (~8): changeState, exit, executeEvent, getSelectedBarPlayConfig
- [ ] **59e:** MainState trait overrides for sound (6)
- [ ] **59f:** Skin stubs MainState/MainController wiring (5)

## Phase 60: PlayerResource Wiring + Result Stubs (~12)

- [ ] **60a:** PlayerResource reverse lookup (2): get_reverse_lookup_data/levels
- [ ] **60b:** PlayerResourceAccess trait expansion (3): get_replay_data_mut, reload_bms_file, set_gauge_option
- [ ] **60c:** Result crate remaining stubs (4): getInputProcessor, irSendStatus
- [ ] **60d:** ChartReplicationMode::Replay* (1)
- [ ] **60e:** MusicSelector remaining stubs (4): bar_renderer, skin_distribution_graph

## Phase 61: OBS + LR2 Skin + External + Modmenu + Decide (~14)

- [ ] **61a:** OBS delayed triggerStateChange (1)
- [ ] **61b:** OBS WebSocket reconnection (1)
- [ ] **61c:** LR2 skin CSV INCLUDE directive (1)
- [ ] **61d:** LR2 skin_loader CSV format support (3)
- [ ] **61e:** External property factories (3)
- [ ] **61f:** Pomyu chara loader (1)
- [ ] **61g:** Modmenu stubs (2): get_selected_bar, get_reverse_lookup_data
- [ ] **61h:** Decide stubs (2): getInputProcessor, play_sound

## Phase 62: Launcher egui Integration (~18)

- [ ] **62a:** SongDatabaseAccessor wiring for editor views (4)
- [ ] **62b:** Configuration view init (3): audio, obs, discord
- [ ] **62c:** SkinConfiguration/KeyConfiguration create+render (4)
- [ ] **62d:** PlayConfigurationView (3): What's New, LR2 import, render
- [ ] **62e:** Remaining launcher stubs (4): table_editor, spinner_cell, course_editor, folder_editor

---

## Minor Unported Items

| Item | Impact | Notes |
|------|--------|-------|
| `BMSModel.compareTo()` | Low | Ord impl on demand. Unused in Java |
| `BMSModelUtils.getAverageNotesPerTime()` | Low | Dead code in Java |
| Skill rating calculation | Low | No Java source (no porting source) |

## Permanent Stubs

- **Twitter4j** (`beatoraja-external`): ~446 lines, `bail!()` — API deprecated, intentionally unimplemented
- **ShortDirectPCM** (`beatoraja-audio`): Java-specific DirectBuffer — unnecessary in Rust
- **JavaFX find_parent_by_class_simple_name** (`beatoraja-launcher`): No egui equivalent
- **randomtrainer.dat** (`beatoraja-modmenu`): Binary resource from Java, uses empty HashMap fallback

## Post-Phase 62 Remaining (blocked)

- ContextMenuBar group (~10): Requires MusicSelector + BarManager deep integration
- Download processors (~5): MainController download processor wiring (circular dep)
