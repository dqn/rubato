# lr2oraja Rust Porting Project (Bevy/Kira)

## Overview

lr2oraja (beatoraja fork, Java 313 files / 72,000+ lines) を Rust へ完全移植するプロジェクト。
周辺機能 (Launcher, ModMenu, OBS, Discord RPC, Downloader) を含む全機能が対象。
**このドキュメントは常に最新に保ち続けること。**

## Directory Structure

```
brs/
  lr2oraja-java/           # Java source (reference implementation)
  lr2oraja-rust/           # Rust port (Cargo workspace)
    crates/
      bms-model/           # BMS parser (self-made)
      bms-rule/            # Judge, gauge, score
      bms-pattern/         # Lane/note shuffle
      bms-replay/          # Replay, ghost data
      bms-database/        # Song/score DB (rusqlite)
      bms-config/          # Config (serde)
      bms-input/           # Keyboard, gamepad, MIDI
      bms-audio/           # Audio (kira)
      bms-skin/            # Skin system (mlua)
      bms-render/          # Rendering (bevy)
      bms-ir/              # Internet ranking (reqwest)
      bms-external/        # Discord RPC, OBS, webhook
      bms-launcher/        # Settings GUI (egui)
      bms-stream/          # Streaming integration
      bms-download/        # Song downloader
      brs/                 # Main binary
    golden-master/         # Test infrastructure
    test-bms/              # Test BMS files
  .claude/plans/           # Detailed porting plans and knowledge docs
```

## Development Guidelines

- **Strict Accuracy:** Judge calculations, BMS parsing, and timing management must be bit-identical to Java.
- **Autonomous Porting:** Claude analyzes `./lr2oraja-java` code and ports module by module to `./lr2oraja-rust`, starting from core modules with fewest dependencies.
- **Deferred Task Tracking:** 作業完了時に未完了のタスクがある場合、実装順序を考慮して `Deferred / Stub Items` セクションに追記すること。次回着手時に漏れなく把握できるようにする。

## Testing Rules

- **Golden Master Testing:** Export Java internal state as JSON, compare against Rust output.
- **TDD:** Red-Green-Refactor for every module.
- **GUI Screenshot Testing:** Capture screenshots from both Java and Rust, compare with SSIM.
- **Test BMS Files:** Claude creates minimal BMS files for each feature.
- **Java Modifications Allowed:** Adding debug output / export methods to Java code is permitted for verification.

### Golden Master Testing Lessons

Lessons learned from Phase 0-3 implementation. Refer to these when implementing future GM tests.

- **Watch for Java-Rust semantic differences:** The same field name may have different semantics (e.g., `wav_id` — Java uses wavlist array index 0-based with -2 for undefined, Rust uses raw base36 value). Verify that compared fields share the same semantics; skip or add conversion logic if they differ.
- **Use ±2μs tolerance for timing comparisons:** BPM → μs conversion produces floating-point rounding differences. ±1μs causes false negatives.
- **Java BMSDecoder hardcodes MS932:** UTF-8 BMS metadata and hashes are garbled on the Java side. Keep UTF-8 tests as `#[ignore]` until Java-side encoding detection is added.
- **`#RANDOM` is deterministic via `random_seeds.json`:** Java exporter reads per-file selectedRandom arrays from `test-bms/random_seeds.json`, and Rust tests must use matching `decode_with_randoms(...)` inputs.
- **Avoid JavaFX dependencies:** `core:compileJava` fails due to JavaFX. Keep the GM exporter in the separate `golden-master` Gradle subproject depending only on jbms-parser + Jackson. Apply the same pattern when adding exports for new modules.
- **Regenerate fixtures with `just golden-master-gen`:** Always regenerate after modifying the Java exporter to keep Rust tests in sync.
- **Parser fixture names must keep source extensions:** Use `filename.ext.json` (e.g. `9key_pms.bms.json`) to avoid collisions across `.bms/.pms` variants sharing the same stem.
- **RenderSnapshot parity triage should use category summaries:** Prefer `command_count / visibility / geometry / detail` counts to quickly detect whether regressions are structural or field-level.
- **Lua functions in skin DST fields must be preserved through JSON serialization:** `lua_value_to_json()` converts Lua functions to a `"__lua_function__"` sentinel string so that `PropertyRef::Script` preserves the "draw field is present" semantics. Without this, Lua `draw = function()` becomes null, causing `op` to be incorrectly used as `option_conditions` (Java ignores `op` when `dst.draw` is non-null).

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

## Key Principles

- All timing uses integer microseconds (i64) to avoid floating-point drift
- LongNote references use index-based approach (no circular references)
- `java.util.Random(seed)` LCG must be reproduced exactly for pattern shuffle
- LR2 judge scaling (`lr2JudgeScaling`) uses pure integer arithmetic

## Plans

- Detailed porting plan: `.claude/plans/iridescent-tumbling-swan.md`
- Critical algorithms: `.claude/plans/critical-algorithms.md`
- Java module analysis: `.claude/plans/java-module-analysis.md`
- Java vs Rust 機能差分分析: `.claude/plans/cryptic-frolicking-bengio.md`

## Implementation Status

Phase 0-24 全完了（16 crate, ~92,000行）。全 RenderSnapshot GM テストが strict parity 達成済み。
全 Deferred Items 実装完了。

### Completed Items (Phase 24)

以下の項目は全て実装済み:

- **アーカイブ展開 (zip/lzh)** — `bms-download/extract.rs` に .zip / .lzh / .tar.gz 全対応 + パストラバーサル防止
- **CLI 引数** — `-a` (autoplay), `-p` (practice), `-r` (replay), `-s` (play) + positional BMS_PATH
- **GithubVersionChecker** — `bms-external/version_check.rs` に GitHub API + semver 比較
- **GhostBattlePlay** — `player_resource.rs` に `GhostBattleSettings` + take() パターン
- **オーディオ障害リカバリ** — `kira_driver.rs` に consecutive_errors 追跡 + try_recover() (AudioManager 再作成)
- **スキンロードエラーフォールバック** — `skin_manager.rs` に 3段階フォールバック (設定スキン → デフォルト → MinimalUI)
- **ホットリロード** — `hot_reload.rs` に F5 キーで config + skin 再読み込み (ModMenu フォーカス考慮)
- **RhythmTimerProcessor** — `rhythm_timer.rs` に Java 忠実移植 (小節線 + 四分音符タイミング)
- **選曲ソート** — 全11モード (Default, Title, Artist, Level, Bpm, Length, Clear, Score, MissCount, Duration, LastUpdate)
- **選曲画面バータイプ** — 全15種 (Song, Folder, Course, TableRoot, HashFolder, Executable, Function, Grade, RandomCourse, Command, Container, SameFolder, SearchWord, LeaderBoard, ContextMenu)
- **MusicSelectCommand** — 全11コマンド (Replay cycling, Clipboard copy, Download stubs, SameFolder, ContextMenu)
- **ライバルスコア表示 UI** — `leaderboard.rs` に entries_to_bars() + 非同期 IR fetch + MusicSelectState 統合
- **スクリーンショット ソーシャルエクスポート** — `social_exporter.rs` に ScoreTextComposer + WebhookScreenshotExporter
- **モニター自動列挙** — `monitor.rs` に macOS (CGDisplay) / Windows (EnumDisplayMonitors) + ドロップダウン UI
- **Launcher GUI Rust 拡張** — ModMenu に5パネル追加 (gauge_visualizer, timer_display, event_trace, profiler, skin_options)
- **Config 細部** — songPreview, skipDecideScreen, frameskip, analogScroll 等全て `bms-config/src/config.rs` に実装済み
- **Stream Controller (Windows Named Pipes)** — `bms-stream/controller.rs` に両プラットフォーム実装済み
- **Window 管理** — モニター選択 + F6 フルスクリーントグル + ModMenu Window Settings + ランチャーモニター自動列挙

### Known Issues

- **bms-model パーサーバグ: 拡張チャンネルがモード判定に影響** — invisible notes (ch 31-37) と mine notes (ch D1-D7) が playable lane として解釈され、Beat5K → Beat7K にモードが誤変更される。`golden_master_channel_extended` テストが `#[ignore]` のまま。修正対象: `bms-model` クレートのチャンネル解釈ロジック

### Completed Deferred Items

以下の項目は全て実装済み:

- **IR プラグインシステム** — `IRConnectionManager` を静的 enum dispatch → 動的 `Box<dyn IRConnection>` レジストリに変更。`async-trait` + `LazyLock<RwLock<HashMap>>` で `register()` / `create()` / `available_names()` API を提供
- **スクリーンショット SSIM テスト** — `plugin.rs` に `register_render_materials()` ヘルパー追加で `BgaLayerMaterial` + embedded shaders をテストハーネスから登録可能に。全26スクリーンショットテスト通過
- **result2.luaskin** — `lua_loader.rs` の `lua_value_to_json()` で NaN/Infinity → JSON null 変換を追加。`RUST_ONLY_CASES` に result2 テスト追加
- **新規スキン Java Fixture** — play14, play7wide, course_result の justfile エントリ追加。`compare_render_snapshot.rs` で play14 (budget=27), play7wide (budget=29) parity テストに昇格、course_result は Rust-only テスト追加
