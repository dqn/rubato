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

Phase 0-43 全完了（16 crate, ~540,000行）。全 RenderSnapshot GM テストが strict parity 達成済み。
主な完了領域: BMS パーサー/ルール/パターン/リプレイ、MusicSelect UI 全機能（キーボードショートカット・コンテキストメニュー・skin state）、LR2IR・ライバルスコア統合、Config 配線、GhostBattle seed+lane_sequence、オーディオ復旧、GithubVersionChecker、Download コマンド統合、Grade バー構築。
詳細な Phase 別履歴は git log を参照。

## Deferred / Stub Items

- **Download 系コマンドの実行ハンドラ:** CommandResult::DownloadHttp/DownloadCourseHttp/DownloadIpfs は定義・ログ出力済み。実際の bms-download Processor 呼び出し（Bevy Resource 経由の非同期タスクキュー、DB リフレッシュ、IPFS プロトコル）は保留
- **Bar::Executable:** production bar list 構築パスで構築されるがコンパイラが検出不可のため `#[allow(dead_code)]` 維持
- **Windows Named Pipes:** Stream Controller の Windows 対応（Unix domain socket → Named Pipes）

## Known Issues

- **Config/State の clone():** 484箇所（監査済み: 大半は Bevy Resource/Component の要件や設定値の受け渡しで妥当。大規模移行不要）
- **#[allow(dead_code)]:** 全体 80 件（30 ファイル）。内訳: Used in tests ~35件、Parsed for completeness ~5件、TODO/deferred ~10件、コンパイラ検出外 ~5件。Phase 30-43 で大幅削減済み
