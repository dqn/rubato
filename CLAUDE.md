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

- **PomyuCharaLoader** — ポミュキャラスキンはスタブ。`is_pomyu_chara()` が常に `false`。(`bms-skin/src/pomyu_chara_loader.rs`)
- **MovieProcessor** — feature gate (`movie`) で無効化中。`FfmpegMovieProcessor` は実装済み (`bms-render/src/bga/ffmpeg_movie_processor.rs`)
- **BGA スキンレンダリング (Layer overlay)** — ~~BGA base は描画対応済みだが、layer の重ね描画は未対応（multi-entity 化が必要）~~ **完了** — MultiEntity 化 + `BgaLayerMaterial` シェーダーで base/layer/poor を実装済み
- **OBS WebSocket 接続** — ~~`try_connect_once()` がスタブ~~ **完了** — 実 WebSocket 接続 + Hello/Identify ハンドシェイク + ステートマシン化した `connection_task` で送受信対応 (`bms-external/src/obs/client.rs`)
- **ScrollSpeedModifier Add mode** — per-timeline scroll field 未実装のためスタブ (`bms-pattern/src/scroll_speed_modifier.rs`)
- **Stream Controller (非Windows)** — ~~macOS/Linux ではスタブ~~ **完了** — Unix ドメインソケット (`/tmp/beatoraja.sock`) でリッスン、複数クライアント対応 (`bms-stream/src/controller.rs`)
- **Download Task Retry** — ~~Retry ボタン未接続~~ **完了** — `HttpDownloadProcessor::retry_task()` + UI ボタン配線 (`bms-download/src/processor.rs`, `bms-render/src/mod_menu/menus/download_task.rs`)
- **Lua main_state stub** — ランタイムなしのスキン読み込み用 (`bms-skin/src/loader/lua_loader.rs`)
- **LR2 Play Loader Pomyu stubs** — `DST_PM_CHARA_*` プロパティがスタブ (`bms-skin/src/loader/lr2_play_loader.rs`)
- **Music Preview** — ~~Select 画面のプレビュー再生未実装~~ **完了** — `brs/src/preview_music.rs` で実装済み
