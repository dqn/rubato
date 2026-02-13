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

## Implementation Status

Phase 0-23 全完了（16 crate, ~61,000行）。全 RenderSnapshot GM テストが strict parity 達成済み。

### Deferred / Stub Items

- **MovieProcessor** — `ffmpeg-next` による動画デコード実装済み。`movie` cargo feature でゲーティング（default 有効）。(`bms-render/src/bga/ffmpeg_movie_processor.rs`)
- **PomyuCharaLoader** — ポミュキャラスキンはスタブ。`is_pomyu_chara()` が常に `false`。(`bms-skin/src/pomyu_chara_loader.rs`)
- **Skin Object Rendering (Loader Wiring)** — レンダリングインフラ（マルチエンティティ基盤、プロシージャルテクスチャパイプライン、draw モジュール）は実装済みだが、一部ローダーが新フィールドにデータを入れないため描画されない。以下の順序で実装すること:
  1. ~~**SkinNumber ローダー (JSON)**~~ — ✅ 完了。`try_build_number()` で `source_images` → `split_grid()` → `build_number_source_set()` → `digit_sources` ポピュレート。`SkinSourceSet` は `ImageRegion` ベースに変更済み。
  2. ~~**SkinFloat ローダー (JSON)**~~ — ✅ 完了。`try_build_float()` で `build_float_source_set()` により 26/24/22/12/11-frame パターン対応。
  3. ~~**SkinNumber/SkinFloat ローダー (LR2 CSV)**~~ — ✅ 完了。`src_number()` で `split_grid()` → `build_number_source_set()` → `digit_sources` ポピュレート。SkinFloat は LR2 CSV に該当なし。
  4. ~~**SkinNumber/SkinFloat negative 画像セット**~~ — ✅ 完了。`build_number_source_set()` / `build_float_source_set()` が `Option<SkinSourceSet>` で negative セットを返す。`minus_digit_sources` フィールドでレンダラーが正負判定して画像切替。
  5. ~~**SkinGauge ローダー**~~ — ✅ 完了。`GaugePart.images` を `Vec<ImageRegion>` に変更。JSON: `try_build_gauge()` で `source_images` → indexmap パターン → `split_grid()` → GaugePart ポピュレート。LR2 CSV: `SRC_GROOVEGAUGE`/`SRC_GROOVEGAUGE_EX`/`DST_GROOVEGAUGE` ハンドラ追加。レンダラーも `texture_rect` 対応済み。(`bms-skin/src/skin_gauge.rs`, `bms-skin/src/loader/json_loader.rs`, `bms-skin/src/loader/lr2_csv_loader.rs`, `bms-render/src/draw/gauge.rs`, `bms-render/src/skin_renderer.rs`)
  6. ~~**SkinJudge ローダー**~~ — ✅ 完了。JSON: `try_build_judge()` で `source_images` を子オブジェクトに接続済み。LR2 CSV: `src_judge()` で lazy creation + `judge_images[slot]` ポピュレート、`dst_judge()` で個別 image.base に DST 適用（OFFSET_JUDGE_*P/OFFSET_LIFT 付き）。`src_nowcombo()`/`dst_nowcombo()` で `judge_counts[slot]` に SkinNumber ポピュレート（relative positioning、center alignment adjustment）。judge ID remap (`5-raw_id` for ≤5) 準拠。(`bms-skin/src/loader/lr2_play_loader.rs`, `bms-skin/src/loader/lr2_csv_loader.rs`)
  - **addJudgeDetail** — Early/Late インジケータと判定 duration 表示は未実装。組み込みテクスチャ `skin/default/judgedetail.png` が必要。
- **SkinBar Rendering / SongInformation Display** — データ構造は移植済みだがレンダリング未接続。`skin_renderer.rs` の catch-all に落ちる。SongInformation も bms-render 側で未使用。
- **IR Submission (ResultState)** — DB 保存後に `tokio::spawn` で fire-and-forget 非同期 IR 送信実装済み。(`brs/src/state/ir_submission.rs`, `brs/src/state/result.rs`)
- **Course IR Submission** — コーススコアの IR 送信実装済み。`CourseData` → `IRCourseData` 変換含む。(`brs/src/state/ir_submission.rs`, `brs/src/state/course_result.rs`)
- **OFFSET_SCRATCHANGLE_1P/2P** — Java `KeyInputProccessor` アルゴリズム忠実移植済み。CW/CCW キー入力によるスムーズ回転。(`brs/src/state/play/play_skin_state.rs`)
- **Per-lane Judge Tracking** — `JudgeManager` に `lane_judge: Vec<i32>` 追加。per-key 判定値が lane 固有値を反映。(`bms-rule/src/judge_manager.rs`, `brs/src/state/play/play_skin_state.rs`)
- **2P Side Properties** — per-key 判定値（`VALUE_JUDGE_2P_*`）、判定タイミング（`VALUE_JUDGE_2P_DURATION`）、判定インジケータ（`OPTION_2P_PERFECT/EARLY/LATE`）、per-player 判定/コンボタイマー（`TIMER_JUDGE_2P`, `TIMER_COMBO_2P`）実装済み。Java `NowJudgeDrawCondition` 準拠。残: `OPTION_2P_0_9`〜`OPTION_2P_100`（ゲージ範囲）、`OPTION_NOW_*_2P`（リアルタイムランク）は Java にも未定義のため対象外。(`bms-rule/src/judge_manager.rs`, `brs/src/state/play/play_skin_state.rs`, `brs/src/state/play/mod.rs`)
